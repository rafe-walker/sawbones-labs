//! Safety invariants over long random sequences of grants, allow-list
//! changes, restarts, clock steps and frames, checked against an independent
//! record of what the sequence so far permits.
//!
//! The record does not parse frames. It lists every read each allow-list
//! permits, byte for byte, and remembers the exact bytes of the leased write.
//! A frame may pass only if it is one of those reads on its port, or the
//! leased write on the lease's port while the lease is live and unspent.

mod common;

use common::{
    action_for, grant, read, replace, rule, try_grant, with_crc, write_multiple, write_single,
};
use sawbones_busguard::{BusGuard, Function::*, Port, Rule, Tick, Verdict, LEASE_TTL_TICKS};

/// A small xorshift generator, so the sequences are the same on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn one_in(&mut self, n: u64) -> bool {
        self.below(n) == 0
    }

    fn byte(&mut self) -> u8 {
        self.next() as u8
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }
}

/// Two allow-lists and the empty one.
fn lists() -> [Vec<Rule>; 3] {
    [
        vec![
            rule(0, 1, ReadHoldingRegisters, 100, 109, 4),
            rule(0, 1, WriteSingleRegister, 200, 201, 1),
            rule(0, 1, WriteMultipleRegisters, 200, 203, 4),
            rule(1, 2, ReadInputRegisters, 0, 7, 8),
            rule(1, 2, WriteSingleRegister, 10, 10, 1),
        ],
        vec![
            rule(1, 2, WriteSingleRegister, 10, 10, 1),
            rule(0, 1, ReadHoldingRegisters, 105, 105, 1),
            rule(0, 1, WriteSingleRegister, 201, 201, 1),
        ],
        vec![],
    ]
}

/// Every read each list permits, as (port, exact bytes), written out from the
/// rules by hand rather than by asking the guard.
fn permitted_reads(list: usize) -> Vec<(u8, Vec<u8>)> {
    let mut reads = Vec::new();
    match list {
        0 => {
            for start in 100u16..=109 {
                for quantity in 1u16..=4 {
                    if start + quantity - 1 <= 109 {
                        reads.push((0, read(1, 0x03, start, quantity)));
                    }
                }
            }
            for start in 0u16..=7 {
                for quantity in 1u16..=8 {
                    if start + quantity - 1 <= 7 {
                        reads.push((1, read(2, 0x04, start, quantity)));
                    }
                }
            }
        }
        1 => reads.push((0, read(1, 0x03, 105, 1))),
        _ => {}
    }
    reads
}

/// Writes to choose leases and frames from, with the ports each list permits
/// them on.
fn write_pool() -> Vec<(Vec<u8>, [&'static [u8]; 3])> {
    vec![
        (write_single(1, 200, 0x1234), [&[0], &[], &[]]),
        (write_single(1, 201, 0x0001), [&[0], &[0], &[]]),
        (write_single(1, 202, 0x0001), [&[], &[], &[]]),
        (write_single(2, 10, 0x00FF), [&[1], &[1], &[]]),
        (write_single(2, 11, 0x00FF), [&[], &[], &[]]),
        (write_multiple(1, 200, &[1, 2, 3, 4]), [&[0], &[], &[]]),
        (write_multiple(1, 201, &[7, 7]), [&[0], &[], &[]]),
        (write_multiple(1, 201, &[7, 7, 7, 7]), [&[], &[], &[]]),
        (write_single(0, 200, 0x1234), [&[], &[], &[]]),
    ]
}

/// What the record expects of the lease.
#[derive(Clone)]
struct ModelLease {
    id: u64,
    port: u8,
    bytes: Vec<u8>,
    expires_at: Tick,
}

/// The independent record.
struct Model {
    list: usize,
    reads: Vec<(u8, Vec<u8>)>,
    lease: Option<ModelLease>,
    last_now: Option<Tick>,
    next_id: u64,
    passes_per_lease: Vec<u32>,
}

impl Model {
    fn new() -> Model {
        Model {
            list: 2,
            reads: permitted_reads(2),
            lease: None,
            last_now: None,
            next_id: 0,
            passes_per_lease: Vec::new(),
        }
    }

    /// Ends the lease on a backward clock or at expiry. True if the clock
    /// went backwards.
    fn observe(&mut self, now: Tick) -> bool {
        let backwards = self.last_now.is_some_and(|last| now < last);
        self.last_now = Some(now);
        if backwards || self.lease.as_ref().is_some_and(|l| now >= l.expires_at) {
            self.lease = None;
        }
        backwards
    }
}

/// A port and frame to try: a permitted read, the leased write, a pool
/// write, or any of them damaged, or noise. The port is usually the one the
/// frame belongs on.
fn next_frame(
    rng: &mut Rng,
    model: &Model,
    all_reads: &[(u8, Vec<u8>)],
    pool: &[(Vec<u8>, [&[u8]; 3])],
) -> (u8, Vec<u8>) {
    let (home, base) = match rng.below(4) {
        0 if !model.reads.is_empty() => rng.pick(&model.reads).clone(),
        1 => match &model.lease {
            Some(lease) => (lease.port, lease.bytes.clone()),
            None => (0, rng.pick(pool).0.clone()),
        },
        2 => rng.pick(all_reads).clone(),
        _ => (rng.below(2) as u8, rng.pick(pool).0.clone()),
    };
    let port = if rng.one_in(4) {
        rng.below(3) as u8
    } else {
        home
    };
    let bytes = mutate(rng, base);
    (port, bytes)
}

/// Usually returns `base` unchanged; otherwise damages it or replaces it with
/// noise.
fn mutate(rng: &mut Rng, base: Vec<u8>) -> Vec<u8> {
    match rng.below(9) {
        // Unchanged.
        0..=3 => base,
        // One bit flipped, CRC left alone.
        4 => {
            let mut bytes = base;
            let index = rng.below(bytes.len() as u64) as usize;
            bytes[index] ^= 1 << rng.below(8);
            bytes
        }
        // One body byte replaced and the CRC recomputed: well-formed but
        // different.
        5 => {
            let mut body = base[..base.len() - 2].to_vec();
            let index = rng.below(body.len() as u64) as usize;
            body[index] = rng.byte();
            with_crc(&body)
        }
        // Cut short or extended.
        6 => {
            let mut bytes = base;
            if rng.one_in(2) {
                bytes.truncate(rng.below(bytes.len() as u64) as usize);
            } else {
                for _ in 0..=rng.below(3) {
                    bytes.push(rng.byte());
                }
            }
            bytes
        }
        // Noise with a correct CRC, so it gets past the CRC check.
        7 => {
            let len = 2 + rng.below(20) as usize;
            let body: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
            with_crc(&body)
        }
        // Plain noise, up to past the longest frame.
        _ => {
            let len = rng.below(300) as usize;
            (0..len).map(|_| rng.byte()).collect()
        }
    }
}

#[test]
fn random_sequences_pass_only_what_was_approved() {
    const SEEDS: u64 = 150;
    const STEPS: usize = 2_000;
    let lists = lists();
    let pool = write_pool();
    let all_reads = permitted_reads(0);
    let mut leased_passes = 0u64;
    let mut read_passes = 0u64;

    for seed in 1..=SEEDS {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let mut guard = BusGuard::new();
        let mut model = Model::new();
        let mut now: Tick = rng.below(1_000);

        for step in 0..STEPS {
            // Time mostly moves forward, sometimes a whole lease lifetime,
            // and now and then backwards.
            now = match rng.below(40) {
                0 => now.saturating_sub(1 + rng.below(LEASE_TTL_TICKS)),
                1 => now + LEASE_TTL_TICKS,
                _ => now + rng.below(6),
            };
            let context = format!("seed {seed} step {step} now {now}");

            match rng.below(30) {
                0 => {
                    guard.restart();
                    model.lease = None;
                    model.last_now = None;
                }
                1 | 2 => {
                    let list = rng.below(3) as usize;
                    guard.apply(replace(&lists[list]), now).unwrap();
                    model.observe(now);
                    model.list = list;
                    model.reads = permitted_reads(list);
                    model.lease = None;
                }
                3..=6 => {
                    let bytes = rng.pick(&pool).0.clone();
                    let port = if rng.one_in(4) { 2 } else { rng.below(2) as u8 };
                    // No lease can name a write the encoder cannot express,
                    // such as a broadcast: the trust root has nothing to
                    // grant.
                    let Some(event) = try_grant(port, &bytes) else {
                        assert!(action_for(port, &bytes).is_none(), "{context}");
                        continue;
                    };
                    let result = guard.apply(event, now);
                    let backwards = model.observe(now);
                    assert_eq!(result.is_err(), backwards, "{context}");
                    if !backwards {
                        model.lease = Some(ModelLease {
                            id: model.next_id,
                            port,
                            bytes,
                            expires_at: now.saturating_add(LEASE_TTL_TICKS),
                        });
                        model.passes_per_lease.push(0);
                        model.next_id += 1;
                    }
                }
                _ => {
                    let (port, bytes) = next_frame(&mut rng, &model, &all_reads, &pool);
                    let decision = guard.decide(Port(port), &bytes, now).verdict();
                    model.observe(now);

                    let is_permitted_read = model
                        .reads
                        .iter()
                        .any(|(p, read)| *p == port && *read == bytes);
                    let leased = model.lease.clone().filter(|lease| {
                        lease.port == port
                            && lease.bytes == bytes
                            && pool.iter().any(|(write, ports)| {
                                *write == bytes && ports[model.list].contains(&port)
                            })
                    });
                    let context = format!("{context} port {port} frame {bytes:02X?}");

                    match decision {
                        Verdict::Pass if is_permitted_read => read_passes += 1,
                        Verdict::Pass => {
                            let lease = leased
                                .unwrap_or_else(|| panic!("passed without approval: {context}"));
                            let spent = &mut model.passes_per_lease[lease.id as usize];
                            *spent += 1;
                            assert_eq!(*spent, 1, "a lease spent twice: {context}");
                            model.lease = None;
                            leased_passes += 1;
                        }
                        Verdict::Block(reason) => {
                            // Nothing approved is refused.
                            assert!(
                                !is_permitted_read,
                                "a permitted read blocked ({reason:?}): {context}"
                            );
                            assert!(
                                leased.is_none(),
                                "the leased write blocked ({reason:?}): {context}"
                            );
                        }
                    }
                    assert_eq!(
                        guard.lease().is_some(),
                        model.lease.is_some(),
                        "lease presence: {context}"
                    );
                }
            }
        }
    }
    // The sequences reached both kinds of pass, so the checks above bit.
    assert!(read_passes > 10_000, "read passes {read_passes}");
    assert!(leased_passes > 500, "leased passes {leased_passes}");
}

#[test]
fn random_bytes_never_pass() {
    // A guard with the widest list and a live lease on every call, fed pure
    // noise of every length up to past the longest frame.
    let lists = lists();
    let reads = permitted_reads(0);
    let leased = write_single(1, 200, 0x1234);
    let mut rng = Rng(0xDEAD_BEEF_CAFE_F00D);
    let mut guard = BusGuard::new();
    guard.apply(replace(&lists[0]), 0).unwrap();
    for i in 0..200_000u64 {
        guard.apply(grant(0, &leased), i).unwrap();
        let len = rng.below(300) as usize;
        let bytes: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        let port = rng.below(2) as u8;
        if guard.decide(Port(port), &bytes, i).is_pass() {
            let approved = reads.iter().any(|(p, r)| *p == port && *r == bytes)
                || (port == 0 && bytes == leased);
            assert!(approved, "noise passed: port {port} {bytes:02X?}");
        }
    }
}

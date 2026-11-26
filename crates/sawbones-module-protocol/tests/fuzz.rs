//! Fuzz-style tests: random and mutated bytes into every decoder, with a
//! fixed seed and a bounded number of cases so CI stays fast and repeatable.
//! Each input must decode or return a typed error, never panic, and anything
//! that decodes must re-encode to exactly the input (one encoding per
//! message).

mod common;

use common::*;
use sawbones_module_protocol::*;

const RANDOM_CASES: usize = 40_000;
const MUTATION_CASES: usize = 40_000;

fn check(bytes: &[u8]) {
    if let Ok(frame) = decode_request(bytes) {
        assert_eq!(request_bytes(frame.header, frame.request), bytes);
    }
    if let Ok(frame) = decode_response(bytes) {
        let frame = frame.into_claim();
        assert_eq!(response_bytes(frame.header, frame.response), bytes);
    }
    let _ = frame_len(bytes);
}

#[test]
fn random_bytes_never_panic_the_decoders() {
    let mut rng = Rng::new(0x0098_f001);
    let mut buf = [0u8; MAX_FRAME_LEN + 16];
    for _ in 0..RANDOM_CASES {
        let len = rng.below(buf.len() as u64 + 1) as usize;
        rng.fill(&mut buf[..len]);
        // Half the time, give it a valid prefix and check value so the
        // payload decoders are reached.
        if rng.bool() && len >= MIN_FRAME_LEN {
            buf[..2].copy_from_slice(&MAGIC);
            buf[2] = VERSION;
            let payload = (len - MIN_FRAME_LEN) as u16;
            buf[4..6].copy_from_slice(&payload.to_le_bytes());
            fix_crc(&mut buf[..len]);
        }
        check(&buf[..len]);
    }
}

#[test]
fn mutated_valid_frames_never_panic_and_never_decode_to_something_else() {
    let mut rng = Rng::new(0x0098_f002);
    let mut pool = [0u8; MAX_CHUNK_LEN];
    let mut accepted_mutants = 0usize;
    for i in 0..MUTATION_CASES {
        rng.fill(&mut pool);
        let mut log_buf = [0u8; MAX_LOG_BYTES];
        let original = if i % 2 == 0 {
            let request = random_request(&mut rng, &pool);
            request_bytes(
                RequestHeader {
                    session: rng.next(),
                    seq: rng.next() as u32,
                },
                request,
            )
        } else {
            let response = random_response(&mut rng, &pool, &mut log_buf);
            let header = ResponseHeader {
                session: rng.next(),
                seq: rng.next() as u32,
                reply_to: rng.next() as u32,
            };
            response_bytes(header, response)
        };
        let mut mutant = original.clone();
        match rng.below(4) {
            0 => {
                let at = rng.below(mutant.len() as u64) as usize;
                mutant[at] ^= 1 << rng.below(8);
            }
            1 => {
                let at = rng.below(mutant.len() as u64) as usize;
                mutant[at] = rng.byte();
            }
            2 => mutant.truncate(rng.below(mutant.len() as u64) as usize),
            _ => mutant.push(rng.byte()),
        }
        let crc_fixed = rng.bool();
        if crc_fixed {
            fix_crc(&mut mutant);
        }
        if !crc_fixed && mutant != original {
            // Without a fixed check value, no single-byte change decodes.
            assert!(decode_request(&mutant).is_err());
            assert!(decode_response(&mutant).is_err());
        }
        if decode_request(&mutant).is_ok() || decode_response(&mutant).is_ok() {
            accepted_mutants += 1;
        }
        check(&mutant);
    }
    // The check value is doing work: some mutants with a fixed check decode
    // (to their own, different message), most do not.
    assert!(accepted_mutants < MUTATION_CASES);
}

#[test]
fn random_bytes_never_panic_the_links() {
    let mut rng = Rng::new(0x0098_f003);
    let mut module = SimModule::new(MODULE_SEED, MODULE);
    let mut host = HostLink::new();
    let mut out = [0u8; MAX_FRAME_LEN];
    host.open(Challenge([1; 16]), &mut out).unwrap();
    let mut buf = [0u8; MAX_FRAME_LEN];
    for _ in 0..10_000 {
        let len = rng.below(buf.len() as u64 + 1) as usize;
        rng.fill(&mut buf[..len]);
        if rng.bool() && len >= MIN_FRAME_LEN {
            buf[..2].copy_from_slice(&MAGIC);
            buf[2] = VERSION;
            buf[4..6].copy_from_slice(&((len - MIN_FRAME_LEN) as u16).to_le_bytes());
            fix_crc(&mut buf[..len]);
        }
        let _ = module.handle(&buf[..len], &mut out);
        let _ = host.receive(&buf[..len]);
    }
}

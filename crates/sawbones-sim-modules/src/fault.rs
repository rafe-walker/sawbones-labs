//! Named, seedable fault scenarios.

use sawbones_module_protocol::ModuleId;

use crate::module::ModuleFaults;

/// Which way along the link a fault acts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dir {
    /// Host to module: requests.
    ToModule,
    /// Module to host: replies.
    ToHost,
}

impl Dir {
    pub(crate) const fn index(self) -> usize {
        match self {
            Dir::ToModule => 0,
            Dir::ToHost => 1,
        }
    }
}

/// What goes wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// Flip `flips` distinct bits in every frame.
    BitFlip {
        /// How many bits.
        flips: u8,
    },
    /// Cut every frame short at a random point.
    Truncate,
    /// Lose one frame in `one_in`.
    Drop {
        /// The odds against losing a frame; 1 loses all.
        one_in: u32,
    },
    /// Deliver every frame `copies` more times.
    Duplicate {
        /// Extra copies.
        copies: u8,
    },
    /// Hold a frame back and deliver it after the next one.
    Reorder,
    /// Deliver every frame this many ticks late.
    Delay {
        /// How late.
        ticks: u64,
    },
    /// Flood the line: this many junk or forged-looking frames per tick, on
    /// top of the real ones.
    Babble {
        /// Frames per tick.
        per_tick: u16,
    },
    /// After each frame, deliver again the one that passed `lag` frames ago.
    ReplayOld {
        /// How many frames back; at least 1.
        lag: usize,
    },
    /// Reboot the module, then replay every recorded frame, both ways.
    /// Identify frames are left out: [`Fault::IdentifyReplay`] covers them.
    /// With `weak_session_rng` the module's session ids repeat after the
    /// reboot.
    ReplayAcrossReboot {
        /// The module's session id generator restarts from its seed.
        weak_session_rng: bool,
    },
    /// Replay the host's captured `Identify`, once, which opens a new
    /// session and ends the host's.
    IdentifyReplay,
    /// The module signs an identity that names another module.
    ClaimOtherId {
        /// The id it claims.
        as_id: ModuleId,
    },
    /// The module reports `can_emit` beyond what it signed.
    OverclaimCanEmit,
    /// Rewrite every frame's length field to `declared`, and, if `fix_crc`,
    /// the check value to match, as anyone on the line can.
    Oversize {
        /// The length written into the header.
        declared: u16,
        /// Recompute the CRC afterwards.
        fix_crc: bool,
    },
    /// Say nothing, ever.
    Silent,
    /// The line sticks at `level`: real frames are lost and a run of that
    /// byte arrives each tick.
    StuckLine {
        /// The byte the line is stuck at.
        level: u8,
    },
    /// A fault in the module itself (stuck value, drift and the rest).
    Module(ModuleFaults),
}

/// A named fault with its seed, direction and window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scenario {
    /// A stable name, `snake_case`.
    pub name: &'static str,
    /// Seeds the random choices (which bits, where to cut).
    pub seed: u64,
    /// Which way it acts.
    pub dir: Dir,
    /// The first tick it is active.
    pub from_tick: u64,
    /// The first tick it is no longer active.
    pub until_tick: u64,
    /// What goes wrong.
    pub fault: Fault,
}

impl Scenario {
    /// A scenario active from `from_tick` on, for good.
    #[must_use]
    pub const fn new(
        name: &'static str,
        seed: u64,
        dir: Dir,
        from_tick: u64,
        fault: Fault,
    ) -> Self {
        Scenario {
            name,
            seed,
            dir,
            from_tick,
            until_tick: u64::MAX,
            fault,
        }
    }

    /// The same scenario, ending at `until_tick`.
    #[must_use]
    pub const fn until(mut self, until_tick: u64) -> Self {
        self.until_tick = until_tick;
        self
    }

    /// True while the fault acts, at `now`, on frames going `dir`.
    #[must_use]
    pub fn active(&self, dir: Dir, now: u64) -> bool {
        self.dir == dir && (self.from_tick..self.until_tick).contains(&now)
    }

    /// The module faults this scenario switches on, if it is one.
    #[must_use]
    pub const fn module_faults(&self) -> Option<ModuleFaults> {
        let base = ModuleFaults {
            stuck: false,
            drift_per_tick: 0,
            disconnected: false,
            stale_ticks: 0,
            wrong_identity: None,
            slow_reply_ticks: 0,
            lost_heartbeat: false,
            overclaim_can_emit: false,
        };
        match self.fault {
            Fault::ClaimOtherId { as_id } => Some(ModuleFaults {
                wrong_identity: Some(as_id),
                ..base
            }),
            Fault::OverclaimCanEmit => Some(ModuleFaults {
                overclaim_can_emit: true,
                ..base
            }),
            Fault::Module(faults) => Some(faults),
            _ => None,
        }
    }
}

/// Every link-level scenario, named, all from tick `from_tick` and seeded
/// from `seed`. `other` is the module id the claim-other-id scenario
/// claims. The tests run each one.
#[must_use]
pub fn scenarios(seed: u64, from_tick: u64, other: ModuleId) -> Vec<Scenario> {
    use Dir::{ToHost, ToModule};
    let s = |name, dir, fault| Scenario::new(name, seed, dir, from_tick, fault);
    vec![
        s("bit_flip_to_host", ToHost, Fault::BitFlip { flips: 3 }),
        s("bit_flip_to_module", ToModule, Fault::BitFlip { flips: 3 }),
        s("truncate_to_host", ToHost, Fault::Truncate),
        s("truncate_to_module", ToModule, Fault::Truncate),
        s("drop_all_to_host", ToHost, Fault::Drop { one_in: 1 }),
        s("drop_all_to_module", ToModule, Fault::Drop { one_in: 1 }),
        s("duplicate_to_host", ToHost, Fault::Duplicate { copies: 2 }),
        s(
            "duplicate_to_module",
            ToModule,
            Fault::Duplicate { copies: 2 },
        ),
        s("reorder_to_host", ToHost, Fault::Reorder),
        s("reorder_to_module", ToModule, Fault::Reorder),
        s("delay_past_deadline", ToHost, Fault::Delay { ticks: 500 }),
        s("babble_to_host", ToHost, Fault::Babble { per_tick: 40 }),
        s("babble_to_module", ToModule, Fault::Babble { per_tick: 40 }),
        s(
            "replay_old_to_module",
            ToModule,
            Fault::ReplayOld { lag: 1 },
        ),
        s("replay_old_to_host", ToHost, Fault::ReplayOld { lag: 1 }),
        s(
            "replay_across_reboot",
            ToModule,
            Fault::ReplayAcrossReboot {
                weak_session_rng: false,
            },
        ),
        s(
            "replay_across_reboot_weak_rng",
            ToModule,
            Fault::ReplayAcrossReboot {
                weak_session_rng: true,
            },
        ),
        s("identify_replay", ToModule, Fault::IdentifyReplay),
        s(
            "claim_other_id",
            ToHost,
            Fault::ClaimOtherId { as_id: other },
        ),
        s("overclaim_can_emit", ToHost, Fault::OverclaimCanEmit),
        s(
            "oversized_length",
            ToHost,
            Fault::Oversize {
                declared: u16::MAX,
                fix_crc: true,
            },
        ),
        s("silent_module", ToHost, Fault::Silent),
        s("stuck_line_high", ToHost, Fault::StuckLine { level: 0xFF }),
    ]
}

//! The session store: what the tools read, kept behind handles so a result
//! carries a summary and a handle, never the raw samples.
//!
//! In memory, for one server process, bounded by [`MAX_RECORDS`]. The
//! knowledge store (the chain) is separate work; this is only the session.

use std::collections::BTreeMap;
use std::fmt;

use sawbones_module_protocol::ModuleId;
use sawbones_supervisor::{ManifestPage, Tick};

/// The most records kept. Past it, the oldest is dropped and its handle
/// stops resolving.
pub const MAX_RECORDS: usize = 256;

/// What a handle points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HandleKind {
    /// Samples from `read`.
    Read,
    /// Manifest pages from `describe`.
    Manifest,
}

impl HandleKind {
    const fn prefix(self) -> &'static str {
        match self {
            HandleKind::Read => "rd",
            HandleKind::Manifest => "mf",
        }
    }
}

/// A handle to stored data: host-made, `rd-N` or `mf-N`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Handle {
    kind: HandleKind,
    n: u32,
}

impl Handle {
    /// Parses `rd-N` or `mf-N`, N from 1 to 9 digits with no leading zero.
    /// Anything else is `None`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Handle> {
        let (prefix, digits) = text.split_once('-')?;
        let kind = match prefix {
            "rd" => HandleKind::Read,
            "mf" => HandleKind::Manifest,
            _ => return None,
        };
        if digits.is_empty()
            || digits.len() > 9
            || digits.starts_with('0')
            || !digits.bytes().all(|b| b.is_ascii_digit())
        {
            return None;
        }
        Some(Handle {
            kind,
            n: digits.parse().ok()?,
        })
    }

    /// What it points at.
    #[must_use]
    pub const fn kind(self) -> HandleKind {
        self.kind
    }
}

impl fmt::Display for Handle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.kind.prefix(), self.n)
    }
}

/// One sample a module answered: its claim and the tick it arrived by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    /// The simulated tick the answer was taken at.
    pub tick: Tick,
    /// The value the module claims.
    pub value: i64,
}

/// What a handle holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record {
    /// A `read`.
    Read {
        /// The module.
        module: ModuleId,
        /// The key.
        key: u16,
        /// The answers, oldest first. Module claims.
        samples: Vec<Sample>,
        /// How many questions were asked.
        asked: u32,
    },
    /// A `describe`.
    Manifest {
        /// The module.
        module: ModuleId,
        /// The pages as received, in order. Module bytes.
        pages: Vec<ManifestPage>,
        /// They hash to the digest pinned at enrollment.
        digest_matches: bool,
    },
}

/// The session store.
#[derive(Debug, Default)]
pub struct Store {
    next: u32,
    records: BTreeMap<Handle, (Tick, Record)>,
}

impl Store {
    /// Keeps `record`, taken at `tick`, and returns its handle.
    pub fn put(&mut self, tick: Tick, record: Record) -> Handle {
        self.next = self.next.saturating_add(1).min(999_999_999);
        let kind = match record {
            Record::Read { .. } => HandleKind::Read,
            Record::Manifest { .. } => HandleKind::Manifest,
        };
        let handle = Handle { kind, n: self.next };
        self.records.insert(handle, (tick, record));
        while self.records.len() > MAX_RECORDS {
            let oldest = self
                .records
                .iter()
                .min_by_key(|(h, _)| h.n)
                .map(|(h, _)| *h);
            if let Some(h) = oldest {
                self.records.remove(&h);
            }
        }
        handle
    }

    /// The record behind `handle`, and the tick it was taken at.
    #[must_use]
    pub fn get(&self, handle: Handle) -> Option<(Tick, &Record)> {
        self.records.get(&handle).map(|(t, r)| (*t, r))
    }

    /// How many records are kept.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// True if none are.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Every record, newest first.
    pub fn newest_first(&self) -> impl Iterator<Item = (Handle, Tick, &Record)> {
        let mut all: Vec<_> = self.records.iter().map(|(h, (t, r))| (*h, *t, r)).collect();
        all.sort_by_key(|(h, _, _)| std::cmp::Reverse(h.n));
        all.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_parse_only_in_their_one_form() {
        assert_eq!(
            Handle::parse("rd-1").map(|h| h.to_string()),
            Some("rd-1".into())
        );
        assert_eq!(
            Handle::parse("mf-123456789").map(|h| h.to_string()),
            Some("mf-123456789".into())
        );
        for bad in [
            "",
            "rd",
            "rd-",
            "rd-0",
            "rd-01",
            "rd-1x",
            "rd--1",
            "rd-+1",
            "xx-1",
            "RD-1",
            "rd-1234567890",
            "rd-1 ",
            " rd-1",
            "rd-١",
        ] {
            assert_eq!(Handle::parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn the_store_is_bounded_and_drops_the_oldest() {
        let mut store = Store::default();
        let record = Record::Read {
            module: ModuleId(1),
            key: 0x100,
            samples: Vec::new(),
            asked: 0,
        };
        let first = store.put(0, record.clone());
        for _ in 0..MAX_RECORDS {
            store.put(0, record.clone());
        }
        assert_eq!(store.len(), MAX_RECORDS);
        assert!(store.get(first).is_none());
    }
}

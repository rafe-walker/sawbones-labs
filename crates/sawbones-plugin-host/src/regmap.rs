//! The seam to data-first register maps (#174): a [`RegisterMap`] run as a
//! [`Decoder`], in process.
//!
//! A register map is data, not code, so it needs no sandbox. It goes through
//! the same [`Decoder`] trait and the same [`decode_frames`](crate::decode_frames)
//! conversion as a wasm plugin: one manifest, one set of checks, one way a
//! value becomes an [`Observation`](sawbones_reading::Observation) without a
//! trust tier. The value text matches `RegisterMap::observe` exactly.
//!
//! A frame is a block of big-endian 16-bit words read from `start` of one
//! table. Every readable register lying wholly inside the block is decoded;
//! a frame is all or nothing, as for plugins.

use std::convert::Infallible;

use sawbones_reading::{Ident, UnitBasis};
use sawbones_regmap::{RegisterMap, Table};

use crate::decoder::{
    DecodeFault, Decoder, DecoderManifest, FrameFault, ManifestError, QuantitySpec, RawObservation,
    RawOutcome, MAX_EXPONENT,
};

/// A register map, one table and one block start, as a [`Decoder`].
#[derive(Debug, Clone)]
pub struct RegisterMapDecoder {
    map: RegisterMap,
    table: Table,
    start: u16,
    manifest: DecoderManifest,
    /// Register names, in manifest order.
    names: Vec<String>,
}

impl RegisterMapDecoder {
    /// The manifest lists every readable register of `table`, in address
    /// order, with the map's unit.
    pub fn new(map: RegisterMap, table: Table, start: u16) -> Result<Self, ManifestError> {
        let mut regs: Vec<_> = map
            .registers()
            .iter()
            .filter(|r| r.access().can_read() && r.table() == table)
            .collect();
        regs.sort_by_key(|r| r.address());
        let mut specs = Vec::with_capacity(regs.len());
        let mut names = Vec::with_capacity(regs.len());
        for reg in regs {
            // A loaded map's register names are valid identifiers.
            let quantity = Ident::new(reg.name()).map_err(|_| ManifestError::Empty)?;
            specs.push(QuantitySpec {
                quantity,
                unit: reg.unit().cloned(),
                unit_basis: if reg.unit().is_some() {
                    UnitBasis::DeclaredBySource
                } else {
                    UnitBasis::Unknown
                },
            });
            names.push(reg.name().to_owned());
        }
        let manifest = DecoderManifest::new(specs)?;
        Ok(Self {
            map,
            table,
            start,
            manifest,
            names,
        })
    }

    fn decode_one(&self, frame: &[u8]) -> RawOutcome {
        if !frame.len().is_multiple_of(2) {
            return Err(FrameFault::Declined(DecodeFault::Malformed));
        }
        let words: Vec<u16> = frame
            .chunks_exact(2)
            .map(|w| u16::from_be_bytes([w[0], w[1]]))
            .collect();
        let mut out = Vec::new();
        for (reg, value) in self.map.decode_block(self.table, self.start, &words) {
            let value = value.map_err(|_| FrameFault::Declined(DecodeFault::Malformed))?;
            let quantity = self
                .names
                .iter()
                .position(|n| n == reg.name())
                .and_then(|i| u16::try_from(i).ok())
                .ok_or(FrameFault::Declined(DecodeFault::Unsupported))?;
            let mantissa = i64::try_from(value.mantissa()).map_err(|_| FrameFault::BadValue)?;
            let exponent = i8::try_from(value.scale())
                .ok()
                .filter(|s| *s <= MAX_EXPONENT)
                .map(|s| -s)
                .ok_or(FrameFault::BadValue)?;
            out.push(RawObservation {
                quantity,
                mantissa,
                exponent,
            });
        }
        Ok(out)
    }
}

impl Decoder for RegisterMapDecoder {
    type Error = Infallible;

    fn manifest(&self) -> &DecoderManifest {
        &self.manifest
    }

    fn decode_raw(&self, frames: &[&[u8]]) -> Result<Vec<RawOutcome>, Infallible> {
        Ok(frames.iter().map(|f| self.decode_one(f)).collect())
    }
}

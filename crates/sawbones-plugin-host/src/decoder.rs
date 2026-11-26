//! Decoder output to [`Observation`]: the one host-side conversion every
//! decoder kind goes through.
//!
//! A decoder, whatever runs it, says only "quantity number `q` has the value
//! `mantissa * 10^exponent`". The host owns everything else: which name and
//! unit quantity `q` means (the [`DecoderManifest`]), which source and when
//! (the [`DecodeContext`] of the captured frame) and where the bytes came
//! from (its [`Provenance`]). An [`Observation`] has no trust tier, so the
//! decoder has no way to name one; a tier is attached later, by the host
//! alone, through `Reading::from_observation_host_only`.
//!
//! The [`Decoder`] trait is the place other decoder kinds plug in. A wasm
//! plugin ([`crate::WasmDecoder`]) is one; a data-only decoder, such as a
//! register map (#174), can implement the same trait in process and share
//! the manifest, the checks and the conversion.

use sawbones_reading::{
    DecimalValue, Ident, Observation, Provenance, TimestampQuality, Unit, UnitBasis, UtcNanos,
};

pub use crate::wire::{DecodeFault, RawObservation, RunFault};

/// Most quantities one manifest may list.
pub const MAX_QUANTITIES: usize = 1024;

/// Largest exponent magnitude accepted. Scales on real devices are a few
/// powers of ten; anything beyond this is treated as garbage.
pub const MAX_EXPONENT: i8 = 24;

/// What one quantity number means. Held by the host, never by the plugin.
#[derive(Debug, Clone, PartialEq)]
pub struct QuantitySpec {
    /// The quantity's name, for example `battery_voltage`.
    pub quantity: Ident,
    /// Its unit, if known.
    pub unit: Option<Unit>,
    /// How the unit was arrived at.
    pub unit_basis: UnitBasis,
}

/// The fixed list of quantities a decoder may emit, by index.
#[derive(Debug, Clone, PartialEq)]
pub struct DecoderManifest {
    quantities: Vec<QuantitySpec>,
}

/// A manifest was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ManifestError {
    /// More than [`MAX_QUANTITIES`] entries.
    #[error("a manifest may list at most {MAX_QUANTITIES} quantities")]
    TooMany,
    /// No entries.
    #[error("a manifest must list at least one quantity")]
    Empty,
}

impl DecoderManifest {
    /// Builds a manifest. Index `i` in the list is quantity number `i`.
    pub fn new(quantities: Vec<QuantitySpec>) -> Result<Self, ManifestError> {
        if quantities.is_empty() {
            return Err(ManifestError::Empty);
        }
        if quantities.len() > MAX_QUANTITIES {
            return Err(ManifestError::TooMany);
        }
        Ok(Self { quantities })
    }

    /// The quantity for number `index`, if there is one.
    pub fn get(&self, index: u16) -> Option<&QuantitySpec> {
        self.quantities.get(usize::from(index))
    }

    /// How many quantities the manifest lists.
    pub fn len(&self) -> usize {
        self.quantities.len()
    }

    /// Always false: a manifest has at least one quantity.
    pub fn is_empty(&self) -> bool {
        self.quantities.is_empty()
    }
}

/// Where and when a frame was captured. Set by the host's capture path,
/// never by a decoder.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodeContext {
    /// The source the frame came from.
    pub source_id: Ident,
    /// When the host received the frame.
    pub received_at: UtcNanos,
    /// How far `received_at` can be trusted as the time of measurement.
    pub timestamp_quality: TimestampQuality,
    /// The transport, address and raw bytes behind the frame.
    pub provenance: Provenance,
}

/// One captured frame and its context.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// The bytes, exactly as captured.
    pub bytes: Vec<u8>,
    /// Where and when.
    pub context: DecodeContext,
}

/// What became of one frame.
#[derive(Debug, Clone, PartialEq)]
pub enum FrameOutcome {
    /// Every value checked out. Each is an [`Observation`], with no tier.
    Decoded(Vec<Observation>),
    /// The frame produced no observations, and why. A frame is all or
    /// nothing: one bad value refuses the whole frame.
    Refused(FrameFault),
}

/// Why a frame produced nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameFault {
    /// The decoder declined it.
    Declined(DecodeFault),
    /// The run ended badly (fuel, memory, trap, bad output).
    Run(RunFault),
    /// The decoder named a quantity number the manifest does not have.
    UnknownQuantity(u16),
    /// The value is out of range (exponent beyond [`MAX_EXPONENT`]).
    BadValue,
}

/// A decoder's raw answer for one frame.
pub type RawOutcome = Result<Vec<RawObservation>, FrameFault>;

/// Something that turns captured frames into raw observations against a
/// manifest. Implementations do not build observations themselves:
/// [`decode_frames`] does, so every decoder kind gets the same checks.
pub trait Decoder {
    /// The error for a whole batch (the decoder could not run at all).
    type Error;
    /// The manifest this decoder's quantity numbers index.
    fn manifest(&self) -> &DecoderManifest;
    /// One raw outcome per frame, in order.
    fn decode_raw(&self, frames: &[&[u8]]) -> Result<Vec<RawOutcome>, Self::Error>;
}

/// Runs `decoder` on `frames` and turns its answers into observations, with
/// the checks above.
pub fn decode_frames<D: Decoder>(
    decoder: &D,
    frames: &[Frame],
) -> Result<Vec<FrameOutcome>, D::Error> {
    let bytes: Vec<&[u8]> = frames.iter().map(|f| f.bytes.as_slice()).collect();
    let raw = decoder.decode_raw(&bytes)?;
    Ok(frames
        .iter()
        .zip(raw)
        .map(|(frame, raw)| {
            match raw.and_then(|values| observations(decoder.manifest(), &frame.context, &values)) {
                Ok(obs) => FrameOutcome::Decoded(obs),
                Err(fault) => FrameOutcome::Refused(fault),
            }
        })
        .collect())
}

/// The exact decimal text for `mantissa * 10^exponent`, positional and with
/// at least one digit before the point: `1823`, `182.3`, `-0.05`, `18230`.
/// The same spelling `sawbones-regmap` gives a `Decimal`, so a value decodes
/// to the same text whichever decoder produced it.
pub fn decimal_text(mantissa: i64, exponent: i8) -> String {
    let mut digits = mantissa.unsigned_abs().to_string();
    let sign = if mantissa < 0 { "-" } else { "" };
    if exponent >= 0 {
        if mantissa != 0 {
            digits.extend(std::iter::repeat_n('0', exponent.unsigned_abs().into()));
        }
        return format!("{sign}{digits}");
    }
    let scale = usize::from(exponent.unsigned_abs());
    if digits.len() <= scale {
        let pad = scale + 1 - digits.len();
        digits.insert_str(0, &"0".repeat(pad));
    }
    let (whole, frac) = digits.split_at(digits.len() - scale);
    format!("{sign}{whole}.{frac}")
}

/// Checks and converts one frame's raw values.
pub fn observations(
    manifest: &DecoderManifest,
    context: &DecodeContext,
    values: &[RawObservation],
) -> Result<Vec<Observation>, FrameFault> {
    values
        .iter()
        .map(|raw| {
            let spec = manifest
                .get(raw.quantity)
                .ok_or(FrameFault::UnknownQuantity(raw.quantity))?;
            if raw.exponent.unsigned_abs() > MAX_EXPONENT.unsigned_abs() {
                return Err(FrameFault::BadValue);
            }
            let value = DecimalValue::parse(&decimal_text(raw.mantissa, raw.exponent))
                .map_err(|_| FrameFault::BadValue)?;
            Ok(Observation {
                source_id: context.source_id.clone(),
                quantity: spec.quantity.clone(),
                value,
                unit: spec.unit.clone(),
                unit_basis: spec.unit_basis,
                received_at: context.received_at,
                source_timestamp: None,
                timestamp_quality: context.timestamp_quality,
                provenance: context.provenance.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_text_is_exact_and_parses() {
        assert_eq!(decimal_text(1823, 0), "1823");
        assert_eq!(decimal_text(-1823, -1), "-182.3");
        assert_eq!(decimal_text(-5, -2), "-0.05");
        assert_eq!(decimal_text(5320, -2), "53.20");
        assert_eq!(decimal_text(5, 3), "5000");
        assert_eq!(decimal_text(0, 3), "0");
        assert_eq!(decimal_text(0, -2), "0.00");
        for (m, e) in [(i64::MIN, -MAX_EXPONENT), (i64::MAX, MAX_EXPONENT), (0, 0)] {
            assert!(DecimalValue::parse(&decimal_text(m, e)).is_ok());
        }
    }
}

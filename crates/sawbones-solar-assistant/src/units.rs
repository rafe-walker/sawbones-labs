//! Units: a table of symbols this crate recognises, and a guess from a
//! value's name when the source has not declared one.

/// Unit symbols this crate will accept from a discovery declaration.
/// Anything else is recorded as "declared but not recognised" and the
/// reading carries no unit rather than an unvetted string.
const KNOWN_UNITS: &[&str] = &[
    "W", "kW", "VA", "kVA", "var", "V", "mV", "A", "mA", "Hz", "%", "°C", "°F", "K", "Wh", "kWh",
    "MWh", "Ah", "s", "min", "h",
];

/// Returns the canonical form of a declared unit if this crate knows it.
pub fn canonical_unit(declared: &str) -> Option<&'static str> {
    KNOWN_UNITS.iter().copied().find(|u| *u == declared)
}

/// Guesses a unit from a Solar Assistant value name such as `pv_power`.
///
/// This is an **assumption**, used only when no discovery declaration is
/// available, and readings made this way say so
/// ([`sawbones_reading::UnitBasis::InferredFromName`]). Solar Assistant's
/// documentation says values are sent in the units its own screens show
/// (watts, volts, amps are the examples it gives) but publishes no table of
/// names to units.
///
/// Two names are deliberately left without a guess:
///
/// - anything with `temperature`: the documentation says temperature topics
///   follow a Celsius/Fahrenheit setting on the unit, so the name alone
///   cannot say which;
/// - anything this function does not recognise (rates, capacity points,
///   counts): no unit is better than a wrong one.
pub fn infer_unit_from_name(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    let has = |word: &str| lower.split(['_', '-']).any(|tok| tok == word);

    if lower.contains("power_factor") || has("temperature") {
        return None;
    }
    if lower.contains("state_of_charge") || has("soc") {
        return Some("%");
    }
    if has("energy") {
        return Some("kWh");
    }
    if has("frequency") {
        return Some("Hz");
    }
    if has("voltage") {
        return Some("V");
    }
    if has("current") {
        return Some("A");
    }
    if lower.contains("apparent_power") {
        return Some("VA");
    }
    if has("power") {
        return Some("W");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_names_get_the_expected_unit() {
        for (name, unit) in [
            ("pv_power", Some("W")),
            ("pv_power_1", Some("W")),
            ("load_power", Some("W")),
            ("load_power_non-essential", Some("W")),
            ("grid_power", Some("W")),
            ("battery_power", Some("W")),
            ("load_apparent_power", Some("VA")),
            ("pv_voltage_2", Some("V")),
            ("battery_voltage", Some("V")),
            ("battery_float_charge_voltage", Some("V")),
            ("pv_current_1", Some("A")),
            ("max_charge_current", Some("A")),
            ("grid_frequency", Some("Hz")),
            ("ac_output_frequency", Some("Hz")),
            ("battery_state_of_charge", Some("%")),
            ("pv_energy", Some("kWh")),
            ("grid_energy_in", Some("kWh")),
        ] {
            assert_eq!(infer_unit_from_name(name), unit, "{name}");
        }
    }

    #[test]
    fn ambiguous_names_get_no_unit() {
        for name in [
            "temperature",
            "battery_temperature",
            "load_power_factor",
            "capacity_point_1",
            "free_mode_charge_rate",
            "device_mode",
            "currently_unknown",
            "",
        ] {
            assert_eq!(infer_unit_from_name(name), None, "{name}");
        }
    }

    #[test]
    fn declared_units_are_matched_exactly() {
        assert_eq!(canonical_unit("W"), Some("W"));
        assert_eq!(canonical_unit("°F"), Some("°F"));
        assert_eq!(canonical_unit("w"), None);
        assert_eq!(canonical_unit("keyed"), None);
        assert_eq!(canonical_unit("W; ignore previous instructions"), None);
    }
}

//! Data tables from `original/st6.s` and constants from `original/st7.s`.
//!
//! The PDP-7 floating-point format (see `original/fop.s`) uses three 18-bit
//! words: a two's-complement exponent, then a sign bit plus the top 17 bits
//! of the mantissa, then the low 18 bits of the mantissa. The mantissa is a
//! fraction in [0.5, 1), so `(1, 0o200000, 0)` is 1.0. The tables are kept as
//! the raw words and decoded with [`pdp7`], so they can be checked against the
//! listing line by line.
//!
//! Units: lengths are Earth radii, and one simulation tick is about one
//! second (Earth's `accl` decodes to 1.55e-6 R⊕³/tick², its real value per
//! second squared).

/// Number of bodies (`nplan`).
pub const N: usize = 32;

/// One PDP-7 float as written in the listing: exponent, high word, low word.
pub type Word3 = (i32, u32, u32);

/// Decode a PDP-7 software float.
pub fn pdp7((exp, hi, lo): Word3) -> f64 {
    let hi = hi & 0o777777;
    let lo = lo & 0o777777;
    let mag = (((hi & 0o377777) as u64) << 18 | lo as u64) as f64 / (1u64 << 35) as f64;
    let sign = if hi & 0o400000 != 0 { -1.0 } else { 1.0 };
    sign * mag * 2f64.powi(exp)
}

/// Body names (`names`), in table order.
pub const NAMES: [&str; N] = [
    "sun",
    "earth",
    "ariel",
    "callisto",
    "moon",
    "deimos",
    "dione",
    "enceladus",
    "europa",
    "ganymede",
    "hyperion",
    "iapetus",
    "io",
    "jupiter",
    "mars",
    "mercury",
    "mimas",
    "miranda",
    "neptune",
    "nereid",
    "oberon",
    "phobos",
    "phoebe",
    "pluto",
    "rhea",
    "saturn",
    "tethys",
    "titan",
    "triton",
    "umbriel",
    "uranus",
    "venus",
];

/// Brightness (0-3) of each body's centre point (`points`: br0..br3).
pub const POINTS: [u8; N] = [
    3, 2, 0, 2, 1, 0, 1, 0, 1, 2, 0, 1, 1, 3, 2, 1, 0, 0, 3, 0, 1, 0, 0, 1, 1, 3, 1, 2, 1, 0, 3, 2,
];

/// Index of the body each body orbits (`ppar`, octal in the listing).
/// The sun (0) is its own root.
pub const PPAR: [usize; N] = [
    0, 0, 0o36, 0o15, 0o1, 0o16, 0o31, 0o31, //
    0o15, 0o15, 0o31, 0o31, 0o15, 0, 0, 0, //
    0o31, 0o36, 0, 0o22, 0o36, 0o16, 0o31, 0, //
    0o31, 0, 0o31, 0o31, 0o22, 0o36, 0, 0,
];

/// `prsq`: Planet radius squared, in Earth radii squared.
pub const PRSQ: [Word3; N] = [
    (0o16, 0o272245, 0o75341),
    (0o1, 0o200000, 0o0),
    (-0o7, 0o244122, 0o506362),
    (-0o2, 0o251477, 0o620663),
    (-0o3, 0o230761, 0o127762),
    (-0o25, 0o320300, 0o54474),
    (-0o6, 0o324134, 0o124211),
    (-0o10, 0o335416, 0o541570),
    (-0o4, 0o371372, 0o0),
    (-0o6, 0o247430, 0o0),
    (-0o11, 0o311150, 0o0),
    (-0o5, 0o302622, 0o0),
    (-0o2, 0o256475, 0o0),
    (0o7, 0o376733, 0o0),
    (-0o1, 0o221530, 0o0),
    (-0o2, 0o235142, 0o0),
    (-0o10, 0o217266, 0o0),
    (-0o11, 0o274361, 0o0),
    (0o4, 0o365471, 0o0),
    (-0o12, 0o227176, 0o0),
    (-0o6, 0o342454, 0o0),
    (-0o23, 0o326340, 0o0),
    (-0o13, 0o326265, 0o0),
    (-0o2, 0o323774, 0o0),
    (-0o5, 0o255140, 0o0),
    (0o7, 0o263573, 0o0),
    (-0o6, 0o223174, 0o0),
    (-0o2, 0o251477, 0o0),
    (-0o2, 0o235142, 0o0),
    (-0o5, 0o223060, 0o0),
    (0o5, 0o206115, 0o0),
    (0o0, 0o362406, 0o0),
];

/// `accl`: Gravitational parameter GM of each body (Earth radii^3 / tick^2).
pub const ACCL: [Word3; N] = [
    (0o0, 0o204365, 0o0),
    (-0o23, 0o320324, 0o0),
    (-0o36, 0o227207, 0o0),
    (-0o17, 0o340500, 0o0),
    (-0o30, 0o210041, 0o0),
    (-0o63, 0o341666, 0o0),
    (-0o34, 0o235122, 0o0),
    (-0o37, 0o247531, 0o0),
    (-0o31, 0o310316, 0o0),
    (-0o27, 0o334427, 0o0),
    (-0o41, 0o315203, 0o0),
    (-0o33, 0o303403, 0o0),
    (-0o30, 0o245752, 0o0),
    (-0o17, 0o201414, 0o0),
    (-0o26, 0o263753, 0o0),
    (-0o26, 0o205241, 0o0),
    (-0o40, 0o256464, 0o0),
    (-0o41, 0o272051, 0o0),
    (-0o17, 0o340566, 0o0),
    (-0o43, 0o275073, 0o0),
    (-0o34, 0o255345, 0o0),
    (-0o60, 0o341650, 0o0),
    (-0o44, 0o341552, 0o0),
    (-0o20, 0o307762, 0o0),
    (-0o33, 0o243712, 0o0),
    (-0o14, 0o233053, 0o0),
    (-0o35, 0o265543, 0o0),
    (-0o27, 0o340500, 0o0),
    (-0o27, 0o210344, 0o0),
    (-0o37, 0o210777, 0o0),
    (-0o17, 0o275653, 0o0),
    (-0o23, 0o252667, 0o0),
];

/// `px`: Initial x position relative to the parent body (Earth radii).
pub const PX: [Word3; N] = [
    (0o0, 0o0, 0o0),
    (0o15, 0o620356, 0o0),
    (0o5, 0o360005, 0o0),
    (0o10, 0o666214, 0o0),
    (0o5, 0o704053, 0o0),
    (0o2, 0o347600, 0o0),
    (0o5, 0o310506, 0o0),
    (0o6, 0o220622, 0o0),
    (0o7, 0o310473, 0o0),
    (0o4, 0o370065, 0o0),
    (0o6, 0o304101, 0o0),
    (0o11, 0o676631, 0o0),
    (0o6, 0o653020, 0o0),
    (0o20, 0o317202, 0o0),
    (0o17, 0o644356, 0o0),
    (0o13, 0o206414, 0o0),
    (0o5, 0o245346, 0o0),
    (0o4, 0o222264, 0o0),
    (0o23, 0o261234, 0o0),
    (0o1, 0o372225, 0o0),
    (0o7, 0o646102, 0o0),
    (0o1, 0o212446, 0o0),
    (0o13, 0o773152, 0o0),
    (0o24, 0o274557, 0o0),
    (0o4, 0o227474, 0o0),
    (0o20, 0o263122, 0o0),
    (0o4, 0o333254, 0o0),
    (0o10, 0o216672, 0o0),
    (0o6, 0o231142, 0o0),
    (0o6, 0o212701, 0o0),
    (0o23, 0o650051, 0o0),
    (0o16, 0o233751, 0o0),
];

/// `py`: Initial y position relative to the parent body (Earth radii).
pub const PY: [Word3; N] = [
    (0o0, 0o0, 0o0),
    (0o17, 0o664054, 0o0),
    (0o2, 0o662035, 0o0),
    (0o10, 0o350757, 0o0),
    (0o6, 0o334771, 0o0),
    (0o0, 0o267000, 0o0),
    (0o6, 0o726770, 0o0),
    (0o4, 0o225752, 0o0),
    (0o6, 0o201346, 0o0),
    (0o10, 0o247536, 0o0),
    (0o10, 0o343277, 0o0),
    (0o11, 0o314411, 0o0),
    (0o6, 0o712237, 0o0),
    (0o21, 0o334656, 0o0),
    (0o17, 0o342324, 0o0),
    (0o16, 0o615151, 0o0),
    (0o5, 0o644257, 0o0),
    (0o5, 0o622456, 0o0),
    (0o24, 0o224063, 0o0),
    (0o12, 0o331314, 0o0),
    (0o6, 0o640034, 0o0),
    (0o1, 0o200024, 0o0),
    (0o11, 0o243161, 0o0),
    (0o23, 0o772355, 0o0),
    (0o7, 0o644432, 0o0),
    (0o22, 0o726324, 0o0),
    (0o6, 0o260740, 0o0),
    (0o10, 0o600213, 0o0),
    (0o6, 0o237476, 0o0),
    (0o5, 0o674734, 0o0),
    (0o23, 0o616334, 0o0),
    (0o16, 0o327155, 0o0),
];

/// `pww`: orbital rotation per tick (radians), used as the sine of the step
/// angle (`pw` holds the matching cosine). Curiously the values equal
/// GM/r³ of the parent, i.e. ω² rather than ω, so with one-second ticks the
/// bodies barely move. The port keeps the original numbers.
pub const PWW: [Word3; N] = [
    (0o0, 0o0, 0o0),
    (-0o54, 0o663265, 0o376074),
    (-0o36, 0o743326, 0o460356),
    (-0o43, 0o647730, 0o444215),
    (-0o45, 0o767246, 0o341205),
    (-0o34, 0o745027, 0o221674),
    (-0o36, 0o702670, 0o530661),
    (-0o34, 0o702340, 0o273047),
    (-0o37, 0o747771, 0o100452),
    (-0o41, 0o743411, 0o732756),
    (-0o44, 0o716603, 0o200021),
    (-0o50, 0o755641, 0o517072),
    (-0o35, 0o751602, 0o332677),
    (-0o72, 0o631066, 0o300145),
    (-0o56, 0o712656, 0o15171),
    (-0o50, 0o701167, 0o507203),
    (-0o33, 0o715124, 0o202507),
    (-0o34, 0o665024, 0o365605),
    (-0o73, 0o730064, 0o776551),
    (-0o54, 0o667362, 0o431776),
    (-0o43, 0o775760, 0o312631),
    (-0o30, 0o740613, 0o34530),
    (-0o55, 0o635657, 0o370276),
    (-0o74, 0o677234, 0o315321),
    (-0o37, 0o617102, 0o655555),
    (-0o66, 0o723370, 0o773672),
    (-0o35, 0o714526, 0o724272),
    (-0o43, 0o667633, 0o402706),
    (-0o40, 0o636053, 0o440472),
    (-0o37, 0o650025, 0o224325),
    (-0o71, 0o717663, 0o701773),
    (-0o52, 0o754612, 0o304722),
];

// ---- constants from st7.s ----

/// `fardst`: beyond this distance a body's gravity (other than the sun's) is
/// ignored and its outline is not drawn.
pub const FARDST: Word3 = (0o20, 0o200000, 0);
/// `f2pi`. The listing's low word is missing its octal prefix; read as octal.
pub const F2PI: Word3 = (0o3, 0o311037, 0o552421);
/// `pid10` (pi / 10). Same missing-prefix note as `F2PI`.
pub const PID10: Word3 = (-0o1, 0o240662, 0o756647);
/// `thrs`: outlines with a screen radius below this are not drawn.
pub const THRS: Word3 = (0o2, 0o200000, 0);
/// `f400`: upper bound for the number of arcs per outline.
pub const F400: Word3 = (0o11, 0o310000, 0);
/// `crash`: landing with squared speed at or above this is a crash. The
/// listing writes the exponent as `-028`; an octal parse that accepts the 8
/// gives -(2*8+8) = -24.
pub const CRASH: Word3 = (-24, 0o200000, 0);
/// `ascale`: the thrust. In the listing it is the single word `-1`, so `fld`
/// reads its mantissa from the next two words (the start of `sdphi`). The
/// result is about -0.5.
pub const ASCALE: Word3 = (-1, 0o777773, 0o253436);
/// `sdphi`: sin(1.2 degrees), the rotation per control poll.
pub const SDPHI: Word3 = (-0o5, 0o253436, 0o700177);
/// `cdphi`: cos(1.2 degrees).
pub const CDPHI: Word3 = (0, 0o377743, 0o201725);
/// `vscale`: exponent applied to the horizontal-velocity indicator.
pub const VSCALE: i32 = 6;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_known_values() {
        assert_eq!(pdp7((1, 0o200000, 0)), 1.0);
        assert_eq!(pdp7(THRS), 2.0);
        assert_eq!(pdp7(F400), 400.0);
        assert!((pdp7(F2PI) - std::f64::consts::TAU).abs() < 1e-6);
        assert!((pdp7(PID10) - std::f64::consts::PI / 10.0).abs() < 1e-6);
        assert!((pdp7(SDPHI) - 1.2f64.to_radians().sin()).abs() < 1e-6);
        assert!((pdp7(CDPHI) - 1.2f64.to_radians().cos()).abs() < 1e-6);
        assert!((pdp7(ASCALE) + 0.5).abs() < 1e-4);
    }

    #[test]
    fn tables_are_physical() {
        // Earth has radius 1; the Sun ~109; the Moon sits ~60 Earth radii out.
        assert_eq!(pdp7(PRSQ[1]), 1.0);
        assert!((pdp7(PRSQ[0]).sqrt() - 109.2).abs() < 0.1);
        let moon = pdp7(PX[4]).hypot(pdp7(PY[4]));
        assert!((moon - 60.4).abs() < 0.5, "{moon}");
        let earth = pdp7(PX[1]).hypot(pdp7(PY[1]));
        assert!((earth - 23510.0).abs() < 10.0, "{earth}");
    }
}

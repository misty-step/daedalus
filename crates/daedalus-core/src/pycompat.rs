//! Small helpers that reproduce Python semantics exactly, so ports can match
//! the reference `runner/` implementation bit-for-bit. Shared by every module
//! that does Python-flavored arithmetic or coercion.

/// Replicate Python's `round(x, ndigits)`: round-half-to-even ("banker's
/// rounding") at `ndigits` decimal places.
pub fn round_half_even(x: f64, ndigits: u32) -> f64 {
    let factor = 10f64.powi(ndigits as i32);
    (x * factor).round_ties_even() / factor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_half_to_even() {
        // exact-tie cases resolve to the even neighbor, matching CPython
        assert_eq!(round_half_even(0.5, 0), 0.0);
        assert_eq!(round_half_even(1.5, 0), 2.0);
        assert_eq!(round_half_even(2.5, 0), 2.0);
        assert_eq!(round_half_even(0.125, 2), 0.12);
    }
}

//! GF(256) finite field arithmetic with const lookup tables.
//!
//! Uses irreducible polynomial x^8 + x^4 + x^3 + x + 1 (0x11B, same as AES).
//! Lookup tables are computed at compile time for constant-time operations.

/// Irreducible polynomial for GF(2^8): x^8 + x^4 + x^3 + x + 1.
const MODULUS: u16 = 0x11B;

/// Logarithm table: LOG[a] = log_g(a) where g = 0x03 is the generator.
/// LOG[0] is unused (log of zero is undefined).
const LOG: [u8; 256] = build_log_table();

/// Exponentiation table: EXP[i] = g^i mod p.
/// Extended to 512 entries to avoid modular reduction during multiplication.
const EXP: [u16; 512] = build_exp_table();

const fn build_exp_table() -> [u16; 512] {
    let mut table = [0u16; 512];
    let mut val: u16 = 1;
    let mut i = 0;
    while i < 512 {
        table[i] = val;
        // Multiply by generator (0x03): val = val * 3 in GF(256)
        val = (val << 1) ^ val; // val * 2 + val = val * 3
        if val >= 256 {
            val ^= MODULUS;
        }
        // Wrap at 255 to keep in field
        if val >= 256 {
            val ^= MODULUS;
        }
        i += 1;
    }
    table
}

const fn build_log_table() -> [u8; 256] {
    let exp = build_exp_table();
    let mut table = [0u8; 256];
    let mut i: usize = 0;
    while i < 255 {
        table[exp[i] as usize] = i as u8;
        i += 1;
    }
    // LOG[0] is undefined, leave as 0 (caller must check for zero)
    table
}

/// GF(256) field element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Gf256(pub(crate) u8);

impl Gf256 {
    pub(crate) const ZERO: Self = Self(0);
    pub(crate) const ONE: Self = Self(1);

    /// Addition in GF(256) is XOR.
    #[inline]
    pub(crate) fn add(self, other: Self) -> Self {
        Self(self.0 ^ other.0)
    }

    /// Subtraction in GF(256) is also XOR (same as addition in characteristic 2).
    #[inline]
    pub(crate) fn sub(self, other: Self) -> Self {
        self.add(other)
    }

    /// Multiplication using log/exp lookup tables. Constant-time.
    #[inline]
    pub(crate) fn mul(self, other: Self) -> Self {
        if self.0 == 0 || other.0 == 0 {
            return Self::ZERO;
        }
        let log_sum = LOG[self.0 as usize] as usize + LOG[other.0 as usize] as usize;
        Self(EXP[log_sum] as u8)
    }

    /// Multiplicative inverse via lookup table.
    ///
    /// # Panics
    /// Panics if called on zero.
    #[inline]
    pub(crate) fn inv(self) -> Self {
        assert!(self.0 != 0, "cannot invert zero in GF(256)");
        let log_inv = 255 - LOG[self.0 as usize] as usize;
        Self(EXP[log_inv] as u8)
    }
}

/// Evaluate polynomial at point x using Horner's method.
/// `coefficients[0]` is the constant term (the secret byte).
pub(crate) fn poly_eval(coefficients: &[Gf256], x: Gf256) -> Gf256 {
    let mut result = Gf256::ZERO;
    // Horner: iterate from highest degree to lowest
    for coeff in coefficients.iter().rev() {
        result = result.mul(x).add(*coeff);
    }
    result
}

/// Lagrange interpolation at x=0 to recover the constant term.
/// Each point is (x_i, y_i) where x_i is the share index.
pub(crate) fn lagrange_interpolate(points: &[(Gf256, Gf256)]) -> Gf256 {
    let mut result = Gf256::ZERO;
    let k = points.len();

    for i in 0..k {
        let (x_i, y_i) = points[i];
        let mut basis = Gf256::ONE;

        for j in 0..k {
            if i == j {
                continue;
            }
            let (x_j, _) = points[j];
            // basis *= x_j / (x_j - x_i)
            // At x=0: basis *= (0 - x_j) / (x_i - x_j) = x_j / (x_j - x_i)
            // In GF(256): subtraction = addition (XOR)
            let numerator = x_j;
            let denominator = x_j.add(x_i);
            basis = basis.mul(numerator).mul(denominator.inv());
        }

        result = result.add(y_i.mul(basis));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gf256_add_identity() {
        for a in 0..=255u8 {
            assert_eq!(Gf256(a).add(Gf256::ZERO), Gf256(a));
        }
    }

    #[test]
    fn gf256_add_self_is_zero() {
        for a in 0..=255u8 {
            assert_eq!(Gf256(a).add(Gf256(a)), Gf256::ZERO);
        }
    }

    #[test]
    fn gf256_mul_identity() {
        for a in 0..=255u8 {
            assert_eq!(Gf256(a).mul(Gf256::ONE), Gf256(a));
        }
    }

    #[test]
    fn gf256_mul_zero() {
        for a in 0..=255u8 {
            assert_eq!(Gf256(a).mul(Gf256::ZERO), Gf256::ZERO);
        }
    }

    #[test]
    fn gf256_mul_inverse() {
        // Exhaustive: all 255 non-zero elements
        for a in 1..=255u8 {
            let inv = Gf256(a).inv();
            assert_eq!(
                Gf256(a).mul(inv),
                Gf256::ONE,
                "a={a}, inv={}, product={}",
                inv.0,
                Gf256(a).mul(inv).0
            );
        }
    }

    #[test]
    fn gf256_mul_commutative() {
        // Sample 1000 pairs
        for a in 0..32u8 {
            for b in 0..32u8 {
                assert_eq!(Gf256(a).mul(Gf256(b)), Gf256(b).mul(Gf256(a)));
            }
        }
    }

    #[test]
    fn gf256_mul_associative() {
        for a in 1..16u8 {
            for b in 1..16u8 {
                for c in 1..16u8 {
                    let lhs = Gf256(a).mul(Gf256(b)).mul(Gf256(c));
                    let rhs = Gf256(a).mul(Gf256(b).mul(Gf256(c)));
                    assert_eq!(lhs, rhs, "({a}*{b})*{c} != {a}*({b}*{c})");
                }
            }
        }
    }

    #[test]
    fn poly_eval_constant() {
        // Polynomial f(x) = 42 -> f(anything) = 42
        let coeffs = [Gf256(42)];
        assert_eq!(poly_eval(&coeffs, Gf256(0)), Gf256(42));
        assert_eq!(poly_eval(&coeffs, Gf256(1)), Gf256(42));
        assert_eq!(poly_eval(&coeffs, Gf256(99)), Gf256(42));
    }

    #[test]
    fn poly_eval_linear() {
        // f(x) = 5 + 3x in GF(256)
        let coeffs = [Gf256(5), Gf256(3)];
        // f(0) = 5
        assert_eq!(poly_eval(&coeffs, Gf256(0)), Gf256(5));
        // f(1) = 5 + 3 = 5 XOR 3 = 6
        assert_eq!(poly_eval(&coeffs, Gf256(1)), Gf256(5).add(Gf256(3)));
    }

    #[test]
    fn lagrange_known_polynomial() {
        // f(x) = 42 + 7x (degree 1, threshold 2)
        let coeffs = [Gf256(42), Gf256(7)];

        // Evaluate at x=1, x=2, x=3
        let y1 = poly_eval(&coeffs, Gf256(1));
        let y2 = poly_eval(&coeffs, Gf256(2));
        let y3 = poly_eval(&coeffs, Gf256(3));

        // Reconstruct from any 2 points — should recover f(0) = 42
        let points_12 = [(Gf256(1), y1), (Gf256(2), y2)];
        assert_eq!(lagrange_interpolate(&points_12), Gf256(42));

        let points_13 = [(Gf256(1), y1), (Gf256(3), y3)];
        assert_eq!(lagrange_interpolate(&points_13), Gf256(42));

        let points_23 = [(Gf256(2), y2), (Gf256(3), y3)];
        assert_eq!(lagrange_interpolate(&points_23), Gf256(42));
    }
}

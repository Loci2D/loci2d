// Pure integer arithmetic and deterministic fixed-point mathematical operations.
// Guarantees 100% bit-exact cross-platform determinism (ADR-0007, ADR-0012).

use crate::world::fixed_point::DeterministicVector2;
use fixed::types::I16F16;

/// Pure integer digit-by-digit restoring square root for 64-bit unsigned integers.
/// Guarantees bit-exact identical output on all CPU architectures (x86_64, ARM64, WASM).
#[inline]
pub fn integer_sqrt_u64(val: u64) -> u64 {
    if val == 0 {
        return 0;
    }
    let mut root = 0u64;
    let mut bit = 1u64 << 62; // Highest power of 4 fitting in u64

    while bit > val {
        bit >>= 2;
    }

    let mut remainder = val;
    while bit != 0 {
        if remainder >= root + bit {
            remainder -= root + bit;
            root = (root >> 1) + bit;
        } else {
            root >>= 1;
        }
        bit >>= 2;
    }
    root
}

/// Pure integer digit-by-digit restoring square root for 128-bit unsigned integers.
/// Guarantees bit-exact identical output on all CPU architectures (x86_64, ARM64, WASM).
#[inline]
pub fn integer_sqrt_u128(val: u128) -> u128 {
    if val == 0 {
        return 0;
    }
    let mut root = 0u128;
    let shift = (127 - val.leading_zeros()) & !1;
    let mut bit = 1u128 << shift;

    let mut remainder = val;
    while bit != 0 {
        if remainder >= root + bit {
            remainder -= root + bit;
            root = (root >> 1) + bit;
        } else {
            root >>= 1;
        }
        bit >>= 2;
    }
    root
}

/// Deterministic fixed-point square root for `I16F16` values.
/// Converts the fixed-point representation into integer bits, scales by 2^16,
/// and computes the integer square root to directly yield raw `I16F16` bits.
#[inline]
pub fn fixed_sqrt(val: I16F16) -> I16F16 {
    if val <= I16F16::ZERO {
        return I16F16::ZERO;
    }
    // Scale raw representation (val * 2^16) by 2^16 so integer sqrt returns raw fixed-point bits
    let scaled = (val.to_bits() as u64) << 16;
    let root = integer_sqrt_u64(scaled);
    I16F16::from_bits(root as i32)
}

/// Overflow-safe Euclidean distance between two points in `I16F16` space.
/// Safe across the entire `I16F16` coordinate domain (`[-32,768.0, +32,767.99998]`,
/// diagonal span up to ~92,681.9 units) via `i128`/`u128` intermediates, saturating
/// deterministically to `I16F16::MAX` when the Euclidean distance exceeds `I16F16::MAX`.
#[inline]
pub fn deterministic_distance(a: DeterministicVector2, b: DeterministicVector2) -> I16F16 {
    let dx_raw = (a.x.to_bits() as i128) - (b.x.to_bits() as i128);
    let dy_raw = (a.y.to_bits() as i128) - (b.y.to_bits() as i128);
    let dist_sq_raw = (dx_raw * dx_raw + dy_raw * dy_raw) as u128;

    // Sqrt of raw squared distance directly gives raw I16F16 representation:
    // sqrt((dx * 2^16)^2 + (dy * 2^16)^2) = sqrt(dist^2 * 2^32) = dist * 2^16
    let root_raw = integer_sqrt_u128(dist_sq_raw);
    if root_raw > i32::MAX as u128 {
        I16F16::MAX
    } else {
        I16F16::from_bits(root_raw as i32)
    }
}

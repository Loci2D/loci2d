// Deterministic Fixed-Point Vector2 implementation
// Guarantees 100% bit-exact arithmetic across CPU architectures and OS platforms (ADR-0007).

use crate::network::packets::Vector2 as ProtoVector2;
use fixed::types::I16F16;
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// Fixed-point 2D Vector using 16-bit integer part and 16-bit fractional part (I16F16).
/// Range: -32,768 to +32,767 units with precision ~0.000015 units (65,536 sub-steps per unit).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct DeterministicVector2 {
    pub x: I16F16,
    pub y: I16F16,
}

impl DeterministicVector2 {
    pub const ZERO: Self = Self {
        x: I16F16::ZERO,
        y: I16F16::ZERO,
    };

    pub const ONE: Self = Self {
        x: I16F16::ONE,
        y: I16F16::ONE,
    };

    pub const UNIT_X: Self = Self {
        x: I16F16::ONE,
        y: I16F16::ZERO,
    };

    pub const UNIT_Y: Self = Self {
        x: I16F16::ZERO,
        y: I16F16::ONE,
    };

    #[inline]
    pub const fn new(x: I16F16, y: I16F16) -> Self {
        Self { x, y }
    }

    /// Quantizes an incoming float (e.g. from network MoveIntent) into fixed-point representation.
    /// Safely handles NaN (mapped to 0.0) and saturates ±Infinity / out-of-range floats without panicking.
    #[inline]
    pub fn from_f32(x: f32, y: f32) -> Self {
        #[inline]
        fn quantize(val: f32) -> I16F16 {
            if val.is_nan() {
                I16F16::ZERO
            } else {
                I16F16::saturating_from_num(val)
            }
        }

        Self {
            x: quantize(x),
            y: quantize(y),
        }
    }

    /// Quantizes an incoming double into fixed-point representation directly.
    #[inline]
    pub fn from_f64(x: f64, y: f64) -> Self {
        #[inline]
        fn quantize(val: f64) -> I16F16 {
            if val.is_nan() {
                I16F16::ZERO
            } else {
                I16F16::saturating_from_num(val)
            }
        }

        Self {
            x: quantize(x),
            y: quantize(y),
        }
    }

    /// Converts fixed-point vector to float representation for Protobuf snapshots / client rendering.
    #[inline]
    pub fn to_f32(self) -> (f32, f32) {
        (self.x.to_num::<f32>(), self.y.to_num::<f32>())
    }

    /// Creates a vector from integer coordinates.
    #[inline]
    pub fn from_i32(x: i32, y: i32) -> Self {
        Self {
            x: I16F16::from_num(x),
            y: I16F16::from_num(y),
        }
    }

    /// Converts into a protobuf `Vector2` struct for network transmission.
    #[inline]
    pub fn to_proto(self) -> ProtoVector2 {
        ProtoVector2 {
            x_bits: self.x.to_bits(),
            y_bits: self.y.to_bits(),
        }
    }

    /// Converts from a protobuf `Vector2` reference with quantization.
    #[inline]
    pub fn from_proto(proto: &ProtoVector2) -> Self {
        Self {
            x: I16F16::from_bits(proto.x_bits),
            y: I16F16::from_bits(proto.y_bits),
        }
    }

    /// Calculates Manhattan distance using pure integer fixed-point math.
    #[inline]
    pub fn manhattan_distance(self, other: Self) -> I16F16 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    /// Calculates dot product using 64-bit intermediate arithmetic to prevent fixed-point overflow.
    #[inline]
    pub fn dot(self, other: Self) -> I16F16 {
        let x_prod = (self.x.to_bits() as i64) * (other.x.to_bits() as i64);
        let y_prod = (self.y.to_bits() as i64) * (other.y.to_bits() as i64);
        let sum_raw = (x_prod + y_prod) >> 16;
        let clamped = sum_raw.clamp(i32::MIN as i64, i32::MAX as i64);
        I16F16::from_bits(clamped as i32)
    }

    /// Calculates Euclidean length using deterministic 64-bit square root.
    #[inline]
    pub fn length(self) -> I16F16 {
        crate::world::physics::math::deterministic_distance(Self::ZERO, self)
    }

    /// Calculates Euclidean distance between two vectors using deterministic 64-bit square root.
    #[inline]
    pub fn distance(self, other: Self) -> I16F16 {
        crate::world::physics::math::deterministic_distance(self, other)
    }

    /// Returns normalized unit vector, or `DeterministicVector2::ZERO` if length is zero.
    #[inline]
    pub fn normalize_or_zero(self) -> Self {
        let len = self.length();
        if len == I16F16::ZERO {
            Self::ZERO
        } else {
            Self {
                x: self.x / len,
                y: self.y / len,
            }
        }
    }

    /// Saturating addition to prevent integer overflow at extreme map coordinates.
    #[inline]
    pub fn saturating_add(self, rhs: Self) -> Self {
        Self {
            x: self.x.saturating_add(rhs.x),
            y: self.y.saturating_add(rhs.y),
        }
    }

    /// Saturating subtraction to prevent integer underflow at extreme map coordinates.
    #[inline]
    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self {
            x: self.x.saturating_sub(rhs.x),
            y: self.y.saturating_sub(rhs.y),
        }
    }
}

impl Add for DeterministicVector2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl AddAssign for DeterministicVector2 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for DeterministicVector2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl SubAssign for DeterministicVector2 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Mul<I16F16> for DeterministicVector2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: I16F16) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl MulAssign<I16F16> for DeterministicVector2 {
    #[inline]
    fn mul_assign(&mut self, rhs: I16F16) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl Div<I16F16> for DeterministicVector2 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: I16F16) -> Self::Output {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

impl DivAssign<I16F16> for DeterministicVector2 {
    #[inline]
    fn div_assign(&mut self, rhs: I16F16) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl Neg for DeterministicVector2 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl mlua::UserData for DeterministicVector2 {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        // Expose components as floats for debugging/printing
        methods.add_method("x_float", |_, vec, ()| Ok(vec.x.to_num::<f64>()));
        methods.add_method("y_float", |_, vec, ()| Ok(vec.y.to_num::<f64>()));

        // Expose arithmetic and math. Behind the scenes, everything stays in I16F16.
        methods.add_meta_method(mlua::MetaMethod::Add, |_, vec1, vec2: mlua::AnyUserData| {
            let vec2 = vec2.borrow::<DeterministicVector2>()?;
            Ok(DeterministicVector2::new(vec1.x + vec2.x, vec1.y + vec2.y))
        });

        methods.add_meta_method(mlua::MetaMethod::Sub, |_, vec1, vec2: mlua::AnyUserData| {
            let vec2 = vec2.borrow::<DeterministicVector2>()?;
            Ok(DeterministicVector2::new(vec1.x - vec2.x, vec1.y - vec2.y))
        });

        methods.add_method("distance_to", |_, vec1, vec2: mlua::AnyUserData| {
            let vec2 = vec2.borrow::<DeterministicVector2>()?;
            Ok(vec1.distance(*vec2).to_num::<f64>())
        });

        methods.add_method("length", |_, vec, ()| {
            Ok(vec.length().to_num::<f64>())
        });

        methods.add_method("normalize", |_, vec, ()| {
            Ok(vec.normalize_or_zero())
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_basic_arithmetic() {
        let v1 = DeterministicVector2::from_f32(1.5, 2.0);
        let v2 = DeterministicVector2::from_f32(0.5, -1.0);

        let sum = v1 + v2;
        assert_eq!(sum.to_f32(), (2.0, 1.0));

        let diff = v1 - v2;
        assert_eq!(diff.to_f32(), (1.0, 3.0));

        let neg = -v1;
        assert_eq!(neg.to_f32(), (-1.5, -2.0));
    }

    #[test]
    fn test_vector_scalar_multiplication_and_division() {
        let v = DeterministicVector2::from_f32(2.0, -3.0);
        let factor = I16F16::from_num(2.5);
        let scaled = v * factor;
        assert_eq!(scaled.to_f32(), (5.0, -7.5));

        let mut v_div = DeterministicVector2::from_f32(6.0, -8.0);
        let divisor = I16F16::from_num(2.0);
        assert_eq!((v_div / divisor).to_f32(), (3.0, -4.0));
        v_div /= divisor;
        assert_eq!(v_div.to_f32(), (3.0, -4.0));
    }

    #[test]
    fn test_vector_dot_length_normalize() {
        let v1 = DeterministicVector2::from_f32(3.0, 4.0);
        let v2 = DeterministicVector2::from_f32(2.0, -1.0);

        // dot: 3*2 + 4*(-1) = 2.0
        assert_eq!(v1.dot(v2), I16F16::from_num(2.0));

        // length of (3, 4) = 5.0
        assert_eq!(v1.length(), I16F16::from_num(5.0));

        // distance between (0, 0) and (3, 4) = 5.0
        assert_eq!(
            DeterministicVector2::ZERO.distance(v1),
            I16F16::from_num(5.0)
        );

        // normalize (3, 4) -> (0.6, 0.8)
        let norm = v1.normalize_or_zero();
        assert_eq!(norm.x, I16F16::from_num(3) / I16F16::from_num(5));
        assert_eq!(norm.y, I16F16::from_num(4) / I16F16::from_num(5));

        // normalize ZERO vector returns ZERO
        assert_eq!(
            DeterministicVector2::ZERO.normalize_or_zero(),
            DeterministicVector2::ZERO
        );
    }

    #[test]
    fn test_manhattan_distance() {
        let a = DeterministicVector2::from_f32(1.0, 2.0);
        let b = DeterministicVector2::from_f32(4.0, 6.0);
        let dist = a.manhattan_distance(b);
        assert_eq!(dist, I16F16::from_num(7.0));
    }

    #[test]
    fn test_proto_roundtrip() {
        let original = DeterministicVector2::from_f32(12.75, -8.25);
        let proto = original.to_proto();
        assert_eq!(proto.x_bits, original.x.to_bits());
        assert_eq!(proto.y_bits, original.y.to_bits());

        let restored = DeterministicVector2::from_proto(&proto);
        assert_eq!(restored, original);
    }

    #[test]
    fn test_saturating_operations() {
        let max_val = DeterministicVector2::new(I16F16::MAX, I16F16::MAX);
        let one = DeterministicVector2::ONE;
        let saturated = max_val.saturating_add(one);
        assert_eq!(saturated, max_val);
    }

    #[test]
    fn test_bit_exact_representation() {
        let v = DeterministicVector2::from_f32(1.5, -2.5);
        let bits_x = v.x.to_bits();
        let bits_y = v.y.to_bits();
        // 1.5 in I16F16 is 1 * 65536 + 32768 = 98304
        assert_eq!(bits_x, 98304);
        // -2.5 in I16F16 is -(2 * 65536 + 32768) = -163840
        assert_eq!(bits_y, -163840);
    }

    #[test]
    fn test_from_f32_malformed_inputs() {
        let nan_vec = DeterministicVector2::from_f32(f32::NAN, f32::INFINITY);
        assert_eq!(nan_vec.x, I16F16::ZERO);
        assert_eq!(nan_vec.y, I16F16::MAX);

        let neg_inf_vec = DeterministicVector2::from_f32(f32::NEG_INFINITY, 100000.0);
        assert_eq!(neg_inf_vec.x, I16F16::MIN);
        assert_eq!(neg_inf_vec.y, I16F16::MAX);
    }
}

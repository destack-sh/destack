use destack_core::FxIndexMap;
use smallvec::SmallVec;

use crate::{
    BinaryOperator, CastOperator, Constant, DefinitionTable, Function, Instruction, Intrinsic,
    TargetLayout, Tree, Type, UnaryOperator, Value, ValueDefinition,
};

/// Maximum definition depth inspected by scalar value queries.
const MAX_DEPTH: usize = 6;

/// Maximum definitions inspected by one scalar query.
const QUERY_LIMIT: usize = 100;

impl Value {
    /// Return whether the value is known to differ from zero.
    pub fn is_known_nonzero(
        self,
        function: &Function,
        definitions: &DefinitionTable,
        target: TargetLayout,
        tree: &Tree,
    ) -> bool {
        ValueQuery::new(function, definitions, target, tree).nonzero(self, 0)
    }

    /// Return whether the value is known to be nonnegative.
    pub fn is_known_nonnegative(
        self,
        function: &Function,
        definitions: &DefinitionTable,
        target: TargetLayout,
        tree: &Tree,
    ) -> bool {
        // recognize unsigned integers and inspect the sign bit of signed integers
        let ty = tree.get(function.expect_value_type(self));
        let Some((width, is_signed)) = ty.integer(target.pointer_bits()) else {
            return matches!(ty, Type::Boolean | Type::Character);
        };
        if !is_signed {
            return true;
        }
        if !(1..=128).contains(&width) {
            return false;
        }

        // inspect the known sign bit
        let bits = KnownBits::analyse(self, 0, function, definitions, target, tree);

        bits.zero & (1 << (width - 1)) != 0
    }
}

/// Bits known to be zero or one in one scalar integer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct KnownBits {
    /// Bits known to be zero.
    pub(crate) zero: u128,
    /// Bits known to be one.
    pub(crate) one: u128,
}

impl KnownBits {
    /// Inspect scalar definitions within a bounded query.
    pub(crate) fn analyse(
        value: Value,
        depth: usize,
        function: &Function,
        definitions: &DefinitionTable,
        target: TargetLayout,
        tree: &Tree,
    ) -> Self {
        ValueQuery::new(function, definitions, target, tree).bits(value, depth)
    }

    /// Transfer known bits through one integer operator.
    fn binary(
        self,
        operator: BinaryOperator,
        other: Self,
        width: u16,
        is_signed: bool,
        is_same: bool,
    ) -> Self {
        let mask = u128::MAX >> (128 - width);
        match operator {
            BinaryOperator::And => Self {
                zero: self.zero | other.zero,
                one: self.one & other.one,
            },
            BinaryOperator::Or => Self {
                zero: self.zero & other.zero,
                one: self.one | other.one,
            },
            BinaryOperator::Xor if is_same => Self::constant(0, mask),
            BinaryOperator::Xor => Self {
                zero: (self.zero & other.zero) | (self.one & other.one),
                one: (self.zero & other.one) | (self.one & other.zero),
            },
            BinaryOperator::Add => self.add(other, false, width),
            BinaryOperator::Subtract if is_same => Self::constant(0, mask),
            BinaryOperator::Subtract => self.add(other.invert(), true, width),
            BinaryOperator::Multiply => self.multiply(other, width, is_same),
            BinaryOperator::Divide if is_same => Self::constant(1, mask),
            BinaryOperator::Divide => self.divide(other, width, is_signed),
            BinaryOperator::Remainder if is_same => Self::constant(0, mask),
            BinaryOperator::Remainder => self.remainder(other, width, is_signed),
            BinaryOperator::ShiftLeft
            | BinaryOperator::ShiftRight
            | BinaryOperator::UnsignedShiftRight => self.shift(other, operator, width, is_signed),
            operator => {
                let sign = if is_signed { 1 << (width - 1) } else { 0 };
                let left = self.flip(sign);
                let right = other.flip(sign);
                let left_max = !left.zero & mask;
                let right_max = !right.zero & mask;
                let unequal = (self.one & other.zero) | (self.zero & other.one) != 0;
                let equal = is_same || (self.zero | self.one == mask && self == other);
                let result = match operator {
                    BinaryOperator::Equal => {
                        if equal {
                            Some(true)
                        } else if unequal {
                            Some(false)
                        } else {
                            None
                        }
                    }
                    BinaryOperator::NotEqual => {
                        if equal {
                            Some(false)
                        } else if unequal {
                            Some(true)
                        } else {
                            None
                        }
                    }
                    BinaryOperator::LessThan => {
                        if left_max < right.one {
                            Some(true)
                        } else if is_same || left.one >= right_max {
                            Some(false)
                        } else {
                            None
                        }
                    }
                    BinaryOperator::LessEqual => {
                        if is_same || left_max <= right.one {
                            Some(true)
                        } else if left.one > right_max {
                            Some(false)
                        } else {
                            None
                        }
                    }
                    BinaryOperator::GreaterThan => {
                        if left.one > right_max {
                            Some(true)
                        } else if is_same || left_max <= right.one {
                            Some(false)
                        } else {
                            None
                        }
                    }
                    BinaryOperator::GreaterEqual => {
                        if is_same || left.one >= right_max {
                            Some(true)
                        } else if left_max < right.one {
                            Some(false)
                        } else {
                            None
                        }
                    }
                    _ => unreachable!("integer operator has no known bit transfer"),
                };

                result.map_or(Self::default(), |value| {
                    Self::constant(u128::from(value), 1)
                })
            }
        }
    }

    /// Transfer known bits through integer machine intrinsics.
    fn intrinsic(intrinsic: Intrinsic, arguments: &[Self], width: u16, is_signed: bool) -> Self {
        let mask = u128::MAX >> (128 - width);
        match (intrinsic, arguments) {
            (
                Intrinsic::LeadingZeroCount
                | Intrinsic::TrailingZeroCount
                | Intrinsic::PopulationCount,
                &[bits],
            ) => {
                let (minimum, maximum) = match intrinsic {
                    Intrinsic::LeadingZeroCount => (
                        (bits.zero << (128 - width)).leading_ones(),
                        (bits.one << (128 - width))
                            .leading_zeros()
                            .min(u32::from(width)),
                    ),
                    Intrinsic::TrailingZeroCount => (
                        bits.zero.trailing_ones(),
                        bits.one.trailing_zeros().min(u32::from(width)),
                    ),
                    _ => (
                        bits.one.count_ones(),
                        u32::from(width) - bits.zero.count_ones(),
                    ),
                };

                Self::range(u128::from(minimum), u128::from(maximum), mask)
            }
            (Intrinsic::ByteSwap, &[bits]) => Self {
                zero: bits.zero.swap_bytes() >> (128 - width),
                one: bits.one.swap_bytes() >> (128 - width),
            },
            (Intrinsic::BitReverse, &[bits]) => Self {
                zero: bits.zero.reverse_bits() >> (128 - width),
                one: bits.one.reverse_bits() >> (128 - width),
            },
            (Intrinsic::IsolateLowestOne, &[bits]) => {
                let minimum = bits.zero.trailing_ones();
                let maximum = bits.one.trailing_zeros().min(u32::from(width));
                let high = mask ^ Self::low_mask((maximum + 1).min(u32::from(width)));
                let one = if minimum == maximum && minimum < u32::from(width) {
                    1 << minimum
                } else {
                    0
                };

                Self {
                    zero: bits.zero | high,
                    one,
                }
            }
            (Intrinsic::RotateLeft | Intrinsic::RotateRight, &[bits, amount]) => {
                let mut result = Self::constant(0, mask);
                let mut first = true;
                for shift in 0..width {
                    let shift_bits = u128::from(shift);
                    let shift_mask = u128::from(width - 1);
                    if shift_bits & amount.zero != 0 || (!shift_bits & amount.one & shift_mask) != 0
                    {
                        continue;
                    }
                    let shift = if intrinsic == Intrinsic::RotateRight {
                        (width - shift) % width
                    } else {
                        shift
                    };
                    let bits = Self {
                        zero: ((bits.zero << shift) | (bits.zero >> ((width - shift) % width)))
                            & mask,
                        one: ((bits.one << shift) | (bits.one >> ((width - shift) % width))) & mask,
                    };
                    result = if first { bits } else { result.intersect(bits) };
                    first = false;
                }

                result
            }
            (
                Intrinsic::AddUnchecked
                | Intrinsic::SubUnchecked
                | Intrinsic::MulUnchecked
                | Intrinsic::DivUnchecked
                | Intrinsic::RemUnchecked
                | Intrinsic::ShlUnchecked
                | Intrinsic::ShrUnchecked,
                &[left, right],
            ) => {
                let operator = match intrinsic {
                    Intrinsic::AddUnchecked => BinaryOperator::Add,
                    Intrinsic::SubUnchecked => BinaryOperator::Subtract,
                    Intrinsic::MulUnchecked => BinaryOperator::Multiply,
                    Intrinsic::DivUnchecked => BinaryOperator::Divide,
                    Intrinsic::RemUnchecked => BinaryOperator::Remainder,
                    Intrinsic::ShlUnchecked => BinaryOperator::ShiftLeft,
                    _ => BinaryOperator::ShiftRight,
                };

                left.binary(operator, right, width, is_signed, false)
            }
            (Intrinsic::SatAdd | Intrinsic::SatSub, &[left, right]) => {
                left.saturate(right, intrinsic == Intrinsic::SatAdd, width, is_signed)
            }
            (Intrinsic::Clamp, &[value, lower, upper]) => value
                .minimum(upper, width, is_signed)
                .maximum(lower, width, is_signed),
            (Intrinsic::Midpoint, &[left, right]) => {
                if is_signed {
                    let (left_min, left_max) = left.signed_range(width);
                    let (right_min, right_max) = right.signed_range(width);
                    let low_floor = (left_min & right_min) + ((left_min ^ right_min) >> 1);
                    let high_floor = (left_max & right_max) + ((left_max ^ right_max) >> 1);
                    let low =
                        low_floor + i128::from(low_floor < 0 && (left_min ^ right_min) & 1 != 0);
                    let high =
                        high_floor + i128::from(high_floor < 0 && (left_max ^ right_max) & 1 != 0);

                    Self::range(low as u128 & mask, high as u128 & mask, mask)
                } else {
                    let minimum = (left.one & right.one) + ((left.one ^ right.one) >> 1);
                    let left_max = !left.zero & mask;
                    let right_max = !right.zero & mask;
                    let maximum = (left_max & right_max) + ((left_max ^ right_max) >> 1);

                    Self::range(minimum, maximum, mask)
                }
            }
            (Intrinsic::DivideCeil, &[left, right]) if !is_signed => {
                let minimum = left.one.div_ceil((!right.zero & mask).max(1));
                let maximum = (!left.zero & mask).div_ceil(right.one.max(1));

                Self::range(minimum, maximum, mask)
            }
            (Intrinsic::DivideCeil, &[left, right]) => left.divide_ceil(right, width),
            (Intrinsic::RemainderEuclidean, &[left, right]) => {
                if is_signed {
                    let (minimum, maximum) = right.signed_range(width);
                    let maximum = minimum
                        .unsigned_abs()
                        .max(maximum.unsigned_abs())
                        .saturating_sub(1);

                    Self::range(0, maximum, mask)
                } else {
                    left.remainder(right, width, false)
                }
            }
            (Intrinsic::AbsDiff, &[left, right]) => {
                let sign = if is_signed { 1 << (width - 1) } else { 0 };
                let left = left.flip(sign);
                let right = right.flip(sign);
                let left_max = !left.zero & mask;
                let right_max = !right.zero & mask;
                let minimum = left
                    .one
                    .saturating_sub(right_max)
                    .max(right.one.saturating_sub(left_max));
                let maximum = left
                    .one
                    .abs_diff(right_max)
                    .max(left_max.abs_diff(right.one));

                Self::range(minimum, maximum, mask)
            }
            (Intrinsic::IsMultipleOf, &[left, right]) => {
                let remainder = left.remainder(right, width, is_signed);
                if left.zero == mask || (right.one != 0 && remainder.zero == mask) {
                    Self::constant(1, 1)
                } else if remainder.one != 0 {
                    Self::constant(0, 1)
                } else {
                    Self::default()
                }
            }
            (Intrinsic::Expect, &[value, _]) | (Intrinsic::Transmute, &[value]) => value,
            _ => Self::default(),
        }
    }

    /// Return a mask containing the requested number of low bits.
    fn low_mask(count: u32) -> u128 {
        if count == 0 {
            0
        } else {
            u128::MAX >> (128 - count)
        }
    }

    /// Bound a signed quotient rounded toward positive infinity.
    fn divide_ceil(self, other: Self, width: u16) -> Self {
        let mask = u128::MAX >> (128 - width);
        let (left_min, left_max) = self.signed_range(width);
        let (right_min, right_max) = other.signed_range(width);
        let limit = (mask >> 1) as i128;
        let mut minimum = i128::MAX;
        let mut maximum = i128::MIN;

        // evaluate extrema on both sides of zero without overflowing the dividend
        for left in [left_min, left_max] {
            for right in [right_min, right_max, -1, 1] {
                if right < right_min || right > right_max || right == 0 {
                    continue;
                }

                // bound overflowing negative division by the largest valid quotient
                let value = if left == -limit - 1 && right == -1 {
                    limit
                } else {
                    left / right + i128::from(left % right != 0 && (left < 0) == (right < 0))
                };
                minimum = minimum.min(value);
                maximum = maximum.max(value);
            }
        }

        Self::range(minimum as u128 & mask, maximum as u128 & mask, mask)
    }

    /// Return the bits shared by all integers in one unsigned interval.
    fn range(minimum: u128, maximum: u128, mask: u128) -> Self {
        // retain the common prefix of both interval endpoints
        if minimum > maximum {
            return Self::default();
        }
        let changed = minimum ^ maximum;
        let varying = Self::low_mask(128 - changed.leading_zeros());
        let known = mask & !varying;

        Self {
            zero: !minimum & known,
            one: minimum & known,
        }
    }

    /// Flip the selected bits while retaining their knownness.
    fn flip(self, bits: u128) -> Self {
        Self {
            zero: (self.zero & !bits) | (self.one & bits),
            one: (self.one & !bits) | (self.zero & bits),
        }
    }

    /// Infer the low product bits and bound its unsigned magnitude.
    fn multiply(self, other: Self, width: u16, is_same: bool) -> Self {
        let mask = u128::MAX >> (128 - width);
        let maximum = (!self.zero & mask).checked_mul(!other.zero & mask);
        let mut result = maximum
            .filter(|value| *value <= mask)
            .map_or(Self::default(), |maximum| Self::range(0, maximum, mask));

        // multiply the consecutive known low bits after removing trailing zeros
        let left_zero = self.zero.trailing_ones();
        let right_zero = other.zero.trailing_ones();
        let left_known = (self.zero | self.one).trailing_ones();
        let right_known = (other.zero | other.one).trailing_ones();
        let count =
            (left_zero + right_zero + (left_known - left_zero).min(right_known - right_zero))
                .min(u32::from(width));
        let low_mask = Self::low_mask(count);
        let low = self.one.wrapping_mul(other.one);
        result.zero |= !low & low_mask;
        result.one |= low & low_mask;

        // retain the fixed bits immediately above a square's trailing zeros
        if is_same {
            let bit = 2 * left_zero + 1;
            if bit < u32::from(width) {
                result.zero |= 1 << bit;
                if self.one & (1 << left_zero) != 0 && bit + 1 < u32::from(width) {
                    result.zero |= 1 << (bit + 1);
                }
            }
        }

        result
    }

    /// Bound a quotient using signed or unsigned operand intervals.
    fn divide(self, other: Self, width: u16, is_signed: bool) -> Self {
        let mask = u128::MAX >> (128 - width);
        if !is_signed {
            let maximum = (!self.zero & mask) / other.one.max(1);

            // bound the quotient using the largest possible divisor
            let minimum = self.one / (!other.zero & mask).max(1);

            return Self::range(minimum, maximum, mask);
        }

        // evaluate extrema on each side of the denominator's zero discontinuity
        let (left_min, left_max) = self.signed_range(width);
        let (right_min, right_max) = other.signed_range(width);
        let mut minimum = i128::MAX;
        let mut maximum = i128::MIN;
        for left in [left_min, left_max] {
            for right in [right_min, right_max, -1, 1] {
                if right < right_min || right > right_max || right == 0 {
                    continue;
                }
                let value = left.wrapping_div(right);
                minimum = minimum.min(value);
                maximum = maximum.max(value);
            }
        }

        Self::range(minimum as u128 & mask, maximum as u128 & mask, mask)
    }

    /// Preserve low dividend bits and bound the remainder's magnitude.
    fn remainder(self, other: Self, width: u16, is_signed: bool) -> Self {
        let mask = u128::MAX >> (128 - width);
        let count = other.zero.trailing_ones().min(u32::from(width));
        let low = Self::low_mask(count);
        let mut result = Self {
            zero: self.zero & low,
            one: self.one & low,
        };

        // bound unsigned remainders by both the dividend and divisor
        if !is_signed {
            let maximum = (!self.zero & mask).min((!other.zero & mask).saturating_sub(1));
            let range = Self::range(0, maximum, mask);
            result.zero |= range.zero;
        }
        // preserve the dividend sign for nonzero signed remainders
        else {
            let sign = 1 << (width - 1);
            let magnitude = other.one;
            if other.zero | other.one == mask && magnitude.is_power_of_two() && magnitude < sign {
                let high = mask ^ (magnitude - 1);
                if self.zero & sign != 0 || self.zero & (magnitude - 1) == magnitude - 1 {
                    result.zero |= high;
                } else if self.one & sign != 0 && self.one & (magnitude - 1) != 0 {
                    result.one |= high;
                }
            } else if self.zero & sign != 0 {
                result.zero |= sign;
            } else if self.one & sign != 0 && result.one != 0 {
                result.one |= sign;
            }
        }

        result
    }

    /// Intersect results for every permitted shift count.
    fn shift(self, amount: Self, operator: BinaryOperator, width: u16, is_signed: bool) -> Self {
        let mask = u128::MAX >> (128 - width);
        let sign = 1 << (width - 1);
        let mut result = None;
        for shift in 0..width {
            let value = u128::from(shift);
            if value & amount.zero != 0 || (!value & amount.one) != 0 {
                continue;
            }
            let bits = if operator == BinaryOperator::ShiftLeft {
                Self {
                    zero: ((self.zero << shift) | ((1u128 << shift) - 1)) & mask,
                    one: (self.one << shift) & mask,
                }
            } else {
                let high = mask ^ (mask >> shift);
                let is_arithmetic = operator == BinaryOperator::ShiftRight && is_signed;
                Self {
                    zero: (self.zero >> shift)
                        | if !is_arithmetic || self.zero & sign != 0 {
                            high
                        } else {
                            0
                        },
                    one: (self.one >> shift)
                        | if is_arithmetic && self.one & sign != 0 {
                            high
                        } else {
                            0
                        },
                }
            };
            result = Some(result.map_or(bits, |known: Self| known.intersect(bits)));
        }

        result.unwrap_or_default()
    }

    /// Return the smallest and largest signed values permitted by these bits.
    pub(crate) fn signed_range(self, width: u16) -> (i128, i128) {
        let mask = u128::MAX >> (128 - width);
        let sign = 1 << (width - 1);
        let minimum = self.one | (!self.zero & sign);
        let maximum = (!self.zero & mask & !sign) | (self.one & sign);
        let minimum = ((minimum << (128 - width)) as i128) >> (128 - width);
        let maximum = ((maximum << (128 - width)) as i128) >> (128 - width);

        (minimum, maximum)
    }

    /// Bound a saturating sum or difference in the declared integer range.
    fn saturate(self, other: Self, is_add: bool, width: u16, is_signed: bool) -> Self {
        let mask = u128::MAX >> (128 - width);
        if is_signed {
            let limit = (mask >> 1) as i128;
            let (left_min, left_max) = self.signed_range(width);
            let (right_min, right_max) = other.signed_range(width);
            let (minimum, maximum) = if is_add {
                (
                    left_min.saturating_add(right_min),
                    left_max.saturating_add(right_max),
                )
            } else {
                (
                    left_min.saturating_sub(right_max),
                    left_max.saturating_sub(right_min),
                )
            };

            Self::range(
                minimum.clamp(-limit - 1, limit) as u128 & mask,
                maximum.clamp(-limit - 1, limit) as u128 & mask,
                mask,
            )
        } else {
            let (minimum, maximum) = if is_add {
                (
                    self.one.saturating_add(other.one).min(mask),
                    (!self.zero & mask)
                        .saturating_add(!other.zero & mask)
                        .min(mask),
                )
            } else {
                (
                    self.one.saturating_sub(!other.zero & mask),
                    (!self.zero & mask).saturating_sub(other.one),
                )
            };

            Self::range(minimum, maximum, mask)
        }
    }

    /// Bound the minimum of two operands in their declared ordering.
    fn minimum(self, other: Self, width: u16, is_signed: bool) -> Self {
        let mask = u128::MAX >> (128 - width);
        let sign = if is_signed { 1 << (width - 1) } else { 0 };
        let left = self.flip(sign);
        let right = other.flip(sign);
        let range = Self::range(
            left.one.min(right.one),
            (!left.zero & mask).min(!right.zero & mask),
            mask,
        );

        range.flip(sign)
    }

    /// Bound the maximum of two operands in their declared ordering.
    fn maximum(self, other: Self, width: u16, is_signed: bool) -> Self {
        let mask = u128::MAX >> (128 - width);
        let sign = if is_signed { 1 << (width - 1) } else { 0 };
        let left = self.flip(sign);
        let right = other.flip(sign);
        let range = Self::range(
            left.one.max(right.one),
            (!left.zero & mask).max(!right.zero & mask),
            mask,
        );

        range.flip(sign)
    }

    /// Describe one exact integer.
    fn constant(value: u128, mask: u128) -> Self {
        Self {
            zero: !value & mask,
            one: value & mask,
        }
    }

    /// Retain the bits shared by both possible values.
    fn intersect(self, other: Self) -> Self {
        Self {
            zero: self.zero & other.zero,
            one: self.one & other.one,
        }
    }

    /// Invert each known bit.
    fn invert(self) -> Self {
        Self {
            zero: self.one,
            one: self.zero,
        }
    }

    /// Propagate the possible carries through integer addition.
    fn add(self, other: Self, carry: bool, width: u16) -> Self {
        // bound the sum using the minimum and maximum possible operands
        let mask = u128::MAX >> (128 - width);
        let carry = u128::from(carry);
        let minimum = self.one.wrapping_add(other.one).wrapping_add(carry);
        let maximum = (!self.zero & mask)
            .wrapping_add(!other.zero & mask)
            .wrapping_add(carry);

        // identify carry bits fixed by those bounds
        let carry_zero = !(maximum ^ self.zero ^ other.zero);
        let carry_one = minimum ^ self.one ^ other.one;
        let known = (self.zero | self.one) & (other.zero | other.one) & (carry_zero | carry_one);

        Self {
            zero: !maximum & known & mask,
            one: minimum & known & mask,
        }
    }
}

/// Reuse scalar results and bound work across recursive definitions.
struct ValueQuery<'a> {
    /// The function containing the queried values.
    function: &'a Function,
    /// Definitions and incoming block arguments.
    definitions: &'a DefinitionTable,
    /// The target integer and pointer widths.
    target: TargetLayout,
    /// The MIR types and instructions.
    tree: &'a Tree,

    /// Definitions still permitted by this query.
    remaining: usize,
    /// Known bits indexed by value and recursion depth.
    bits: FxIndexMap<(Value, usize), KnownBits>,
    /// Nonzero results indexed by value and recursion depth.
    nonzero: FxIndexMap<(Value, usize), bool>,
}

impl<'a> ValueQuery<'a> {
    /// Start one query over immutable function definitions.
    fn new(
        function: &'a Function,
        definitions: &'a DefinitionTable,
        target: TargetLayout,
        tree: &'a Tree,
    ) -> Self {
        Self {
            function,
            definitions,
            target,
            tree,
            remaining: QUERY_LIMIT,
            bits: FxIndexMap::default(),
            nonzero: FxIndexMap::default(),
        }
    }

    /// Reuse known bits or inspect one definition within the shared budget.
    fn bits(&mut self, value: Value, depth: usize) -> KnownBits {
        // reuse completed results before charging another definition
        if let Some(&bits) = self.bits.get(&(value, depth)) {
            return bits;
        }
        if depth >= MAX_DEPTH || self.remaining == 0 {
            return KnownBits::default();
        }

        // charge each uncached definition before following its operands
        self.remaining -= 1;
        let bits = self.transfer_bits(value, depth);
        self.bits.insert((value, depth), bits);

        bits
    }

    /// Reuse a nonzero result or inspect one definition within the shared budget.
    fn nonzero(&mut self, value: Value, depth: usize) -> bool {
        // reuse completed results before charging another definition
        if let Some(&nonzero) = self.nonzero.get(&(value, depth)) {
            return nonzero;
        }
        if depth >= MAX_DEPTH || self.remaining == 0 {
            return false;
        }

        // share the work limit with known bit queries made by this definition
        self.remaining -= 1;
        let nonzero = self.transfer_nonzero(value, depth);
        self.nonzero.insert((value, depth), nonzero);

        nonzero
    }

    /// Transfer nonzero guarantees through one definition.
    fn transfer_nonzero(&mut self, value: Value, depth: usize) -> bool {
        // read immutable definitions while updating the query cache
        let function = self.function;
        let definitions = self.definitions;
        let tree = self.tree;

        // retain the nonnull guarantee of references and bounded scalar bit queries
        if matches!(
            tree.get(function.expect_value_type(value)),
            Type::Reference { .. }
        ) {
            return true;
        }
        if self.bits(value, depth).one != 0 {
            return true;
        }

        // require every incoming value to remain nonzero across a merge
        if matches!(
            definitions.definition(value),
            Some(ValueDefinition::BlockParameter { .. })
        ) {
            let mut has_input = false;
            for input in definitions.inputs(value) {
                if self.remaining == 0 {
                    return false;
                }
                let Some(argument) = input.argument else {
                    return false;
                };
                if argument == value {
                    continue;
                }
                if !self.nonzero(argument, depth + 1) {
                    return false;
                }
                has_input = true;
            }

            return has_input;
        }

        // preserve nonzero values through selections and invertible integer operations
        let Some(instruction) = definitions.instruction(value) else {
            return false;
        };
        match tree.get(instruction) {
            Instruction::Copy { value, .. } => self.nonzero(*value, depth + 1),
            Instruction::Select {
                then_value,
                else_value,
                ..
            } => self.nonzero(*then_value, depth + 1) && self.nonzero(*else_value, depth + 1),
            Instruction::Unary {
                operator: UnaryOperator::Negate,
                argument,
                ..
            } => self.nonzero(*argument, depth + 1),
            Instruction::Cast {
                operator: CastOperator::IntToInt,
                argument,
                to_type,
                ..
            } => {
                let source = tree.get(function.expect_value_type(*argument));
                let target = tree.get(*to_type);
                let pointer_bits = self.target.pointer_bits();
                let widths = (source.integer(pointer_bits), target.integer(pointer_bits));

                matches!(widths, (Some((source, _)), Some((target, _))) if target >= source)
                    && self.nonzero(*argument, depth + 1)
            }
            Instruction::Intrinsic {
                intrinsic:
                    Intrinsic::ByteSwap
                    | Intrinsic::BitReverse
                    | Intrinsic::IsolateLowestOne
                    | Intrinsic::RotateLeft
                    | Intrinsic::RotateRight
                    | Intrinsic::Expect,
                arguments,
                ..
            } => {
                let arguments = tree.get_values(*arguments);

                self.nonzero(arguments[0], depth + 1)
            }
            Instruction::Intrinsic {
                intrinsic: Intrinsic::MulUnchecked,
                arguments,
                ..
            } => tree
                .get_values(*arguments)
                .iter()
                .all(|argument| self.nonzero(*argument, depth + 1)),
            Instruction::Intrinsic {
                intrinsic: Intrinsic::PopulationCount,
                arguments,
                ..
            } => self.nonzero(tree.get_values(*arguments)[0], depth + 1),
            Instruction::Intrinsic {
                intrinsic: intrinsic @ (Intrinsic::LeadingZeroCount | Intrinsic::TrailingZeroCount),
                arguments,
                ..
            } => {
                let value = tree.get_values(*arguments)[0];
                let bits = self.bits(value, depth + 1);
                if *intrinsic == Intrinsic::TrailingZeroCount {
                    bits.zero & 1 != 0
                } else {
                    let ty = tree.get(function.expect_value_type(value));
                    ty.integer(self.target.pointer_bits())
                        .is_some_and(|(width, _)| bits.zero & (1 << (width - 1)) != 0)
                }
            }
            _ => false,
        }
    }

    /// Transfer known bits through one definition.
    fn transfer_bits(&mut self, value: Value, depth: usize) -> KnownBits {
        // read immutable definitions while updating the query cache
        let function = self.function;
        let definitions = self.definitions;
        let tree = self.tree;
        let target = self.target;

        // determine the represented integer width
        let ty = tree.get(function.expect_value_type(value));
        let (width, is_signed) = match ty {
            Type::Boolean => (1, false),
            Type::Character => (32, false),
            _ => match ty.integer(self.target.pointer_bits()) {
                Some(integer) => integer,
                None => return KnownBits::default(),
            },
        };
        if !(1..=128).contains(&width) {
            return KnownBits::default();
        }
        let mask = u128::MAX >> (128 - width);

        // retain the unknown caller input even when the entry has backedges
        if matches!(
            definitions.definition(value),
            Some(ValueDefinition::FunctionParameter(_))
        ) {
            return KnownBits::default();
        }

        // intersect incoming definitions of block parameters
        let Some(instruction) = definitions.instruction(value) else {
            let mut result = None;
            for input in definitions.inputs(value) {
                if self.remaining == 0 {
                    return KnownBits::default();
                }
                let Some(argument) = input.argument else {
                    return KnownBits::default();
                };
                if argument == value {
                    continue;
                }
                let bits = self.bits(argument, depth + 1);
                result = Some(result.map_or(bits, |known: KnownBits| known.intersect(bits)));
                if result == Some(KnownBits::default()) {
                    break;
                }
            }

            return result.unwrap_or_default();
        };

        // transfer known bits through integer operations
        let bits = match tree.get(instruction) {
            Instruction::Copy { value, .. } => self.bits(*value, depth + 1),
            Instruction::Const { value, .. } => match value {
                Constant::Int { value, .. } => KnownBits::constant(*value as u128, mask),
                Constant::UInt { value, .. } => KnownBits::constant(*value, mask),
                Constant::Boolean { value } => KnownBits::constant(u128::from(*value), mask),
                Constant::Char { value } => {
                    KnownBits::constant(u128::from(u32::from(*value)), mask)
                }
                Constant::Zeroed => KnownBits::constant(0, mask),
                _ => KnownBits::default(),
            },
            Instruction::Unary {
                operator, argument, ..
            } => {
                let bits = self.bits(*argument, depth + 1);
                match operator {
                    UnaryOperator::Not => KnownBits {
                        zero: bits.one,
                        one: bits.zero,
                    },
                    UnaryOperator::Negate => {
                        KnownBits::constant(0, mask).add(bits.invert(), true, width)
                    }
                }
            }
            Instruction::Binary {
                operator,
                left,
                right,
                ..
            } => {
                let ty = tree.get(function.expect_value_type(*left));
                let (width, is_signed) = match ty {
                    Type::Boolean => (1, false),
                    Type::Character => (32, false),
                    _ => match ty.integer(self.target.pointer_bits()) {
                        Some(integer) => integer,
                        None => return KnownBits::default(),
                    },
                };
                let left_bits = self.bits(*left, depth + 1);
                let right_bits = self.bits(*right, depth + 1);

                left_bits.binary(*operator, right_bits, width, is_signed, left == right)
            }
            Instruction::Cast {
                operator, argument, ..
            } => {
                let bits = self.bits(*argument, depth + 1);
                let source = tree.get(function.expect_value_type(*argument));
                let source_integer = source.integer(target.pointer_bits());
                match (operator, source_integer) {
                    (CastOperator::Bitcast, Some(_)) => bits,
                    (CastOperator::IntToInt, Some((source_width, _))) if source_width >= width => {
                        bits
                    }
                    (CastOperator::IntToInt, Some((source_width, source_signed)))
                        if source_width > 0 =>
                    {
                        let high = mask ^ (u128::MAX >> (128 - source_width));
                        let sign = 1 << (source_width - 1);
                        KnownBits {
                            zero: bits.zero
                                | if !source_signed || bits.zero & sign != 0 {
                                    high
                                } else {
                                    0
                                },
                            one: bits.one
                                | if source_signed && bits.one & sign != 0 {
                                    high
                                } else {
                                    0
                                },
                        }
                    }
                    (CastOperator::IntToIntSaturating, Some((source_width, source_signed))) => {
                        let sign = 1u128 << (width - 1);
                        let maximum = if is_signed { sign - 1 } else { mask };
                        if source_signed {
                            let (lower, upper) = bits.signed_range(source_width);
                            let minimum = if is_signed {
                                (sign as i128).wrapping_neg()
                            } else {
                                0
                            };
                            let maximum = maximum.min(i128::MAX as u128) as i128;

                            KnownBits::range(
                                lower.clamp(minimum, maximum) as u128 & mask,
                                upper.clamp(minimum, maximum) as u128 & mask,
                                mask,
                            )
                        } else {
                            let source_mask = u128::MAX >> (128 - source_width);

                            KnownBits::range(
                                bits.one.min(maximum),
                                (!bits.zero & source_mask).min(maximum),
                                mask,
                            )
                        }
                    }
                    _ => KnownBits::default(),
                }
            }
            Instruction::Select {
                condition,
                then_value,
                else_value,
                ..
            } => {
                let condition = self.bits(*condition, depth + 1);
                let left = self.bits(*then_value, depth + 1);
                let right = self.bits(*else_value, depth + 1);
                if condition.one != 0 {
                    left
                } else if condition.zero & 1 != 0 {
                    right
                } else {
                    left.intersect(right)
                }
            }
            Instruction::Intrinsic {
                intrinsic,
                arguments,
                ..
            } => {
                let arguments = tree.get_values(*arguments);
                let bits = arguments
                    .iter()
                    .map(|argument| self.bits(*argument, depth + 1))
                    .collect::<SmallVec<[_; 3]>>();

                // use the operand integer width for intrinsic transfers
                let integer = arguments.first().and_then(|value| {
                    tree.get(function.expect_value_type(*value))
                        .integer(target.pointer_bits())
                });
                let (width, is_signed) = integer.unwrap_or((width, is_signed));

                KnownBits::intrinsic(*intrinsic, &bits, width, is_signed)
            }
            _ => KnownBits::default(),
        };

        KnownBits {
            zero: bits.zero & mask,
            one: bits.one & mask,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestModule;

    /// Infer nonzero values and signs through masks, shifts, and casts.
    #[test]
    fn test_track_integer_bits() {
        let test = TestModule::new(
            r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = 255
    v3: int32 = or v0, v1
    v4: int32 = and v0, v2
    v5: int32 = ushr v0, v1
    v6: uint8 = cast.intToInt v0 -> uint8
    v7: int32 = cast.intToInt v6 -> int32
    v8: int32 = -1
    return v4
}
"#,
        );
        let function = test.tree.get(test.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &test.tree);
        let target = TargetLayout::default();
        for (index, nonzero, nonnegative) in [
            (0, false, false),
            (1, true, true),
            (2, true, true),
            (3, true, false),
            (4, false, true),
            (5, false, true),
            (6, false, true),
            (7, false, true),
            (8, true, false),
        ] {
            let value = Value(index);
            let actual = (
                value.is_known_nonzero(function, &definitions, target, &test.tree),
                value.is_known_nonnegative(function, &definitions, target, &test.tree),
            );

            assert_eq!(actual, (nonzero, nonnegative), "v{index}");
        }
    }

    /// Distinguish caller inputs from constant loop parameters across backedges.
    #[test]
    fn test_preserve_entry_parameter_bits() {
        let program = TestModule::new(
            r#"
function test(v0: int32, v1: boolean): int32 {
entry(v0: int32, v1: boolean):
    v2: int32 = 1
    branch v1 => entry(v2, v1) | loop(v2)

loop(v3: int32):
    branch v1 => loop(v3) | done

done:
    return v3
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        let target = TargetLayout::default();
        for (index, expected) in [(0, (false, false)), (3, (true, true))] {
            let value = Value(index);
            let actual = (
                value.is_known_nonzero(function, &definitions, target, &program.tree),
                value.is_known_nonnegative(function, &definitions, target, &program.tree),
            );

            assert_eq!(actual, expected, "v{index}");
        }
    }

    /// Bound a masked product and retain the fixed low bit of a square.
    #[test]
    fn test_bound_masked_products() {
        let program = TestModule::new(
            r#"
function test(v0: uint8): void {
entry(v0: uint8):
    v1: uint8 = 15
    v2: uint8 = 3
    v3: uint8 = and v0, v1
    v4: uint8 = mul v3, v2
    v5: uint8 = mul v0, v0
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        for (value, zero, one) in [(4, 0b11000000, 0), (5, 0b00000010, 0)] {
            let actual = KnownBits::analyse(
                Value(value),
                0,
                function,
                &definitions,
                TargetLayout::default(),
                &program.tree,
            );

            assert_eq!(actual, KnownBits { zero, one }, "v{value}");
        }
    }

    /// Bound division and remainder of a masked value by a power of two.
    #[test]
    fn test_bound_power_of_two_quotients_and_remainders() {
        let program = TestModule::new(
            r#"
function test(v0: uint8): void {
entry(v0: uint8):
    v1: uint8 = 15
    v2: uint8 = 4
    v3: uint8 = and v0, v1
    v4: uint8 = rem v3, v2
    v5: uint8 = div v3, v2
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        for (value, zero, one) in [(4, 0b11111100, 0), (5, 0b11111100, 0)] {
            let actual = KnownBits::analyse(
                Value(value),
                0,
                function,
                &definitions,
                TargetLayout::default(),
                &program.tree,
            );

            assert_eq!(actual, KnownBits { zero, one }, "v{value}");
        }
    }

    /// Retain only bits shared by every permitted odd shift count.
    #[test]
    fn test_intersect_variable_shift_results() {
        let program = TestModule::new(
            r#"
function test(v0: uint8, v1: uint8): void {
entry(v0: uint8, v1: uint8):
    v2: uint8 = 15
    v3: uint8 = 3
    v4: uint8 = 1
    v5: uint8 = and v0, v2
    v6: uint8 = and v1, v3
    v7: uint8 = or v6, v4
    v8: uint8 = shl v5, v7
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        let actual = KnownBits::analyse(
            Value(8),
            0,
            function,
            &definitions,
            TargetLayout::default(),
            &program.tree,
        );

        assert_eq!(
            actual,
            KnownBits {
                zero: 0b10000001,
                one: 0
            }
        );
    }

    /// Retain the high bits when a masked value is added near the unsigned maximum.
    #[test]
    fn test_bound_saturated_unsigned_addition() {
        let program = TestModule::new(
            r#"
function test(v0: uint8): void {
entry(v0: uint8):
    v1: uint8 = 15
    v2: uint8 = 240
    v3: uint8 = and v0, v1
    v4: uint8 = intrinsic.math.arithmetic.saturating.add(v3, v2)
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        let actual = KnownBits::analyse(
            Value(4),
            0,
            function,
            &definitions,
            TargetLayout::default(),
            &program.tree,
        );

        assert_eq!(
            actual,
            KnownBits {
                zero: 0,
                one: 0b11110000
            }
        );
    }

    /// Resolve disjoint integer ranges and comparisons of the same SSA value.
    #[test]
    fn test_resolve_integer_comparisons_and_self_subtraction() {
        let program = TestModule::new(
            r#"
function test(v0: uint8): void {
entry(v0: uint8):
    v1: uint8 = 15
    v2: uint8 = 240
    v3: uint8 = and v0, v1
    v4: boolean = lt v3, v2
    v5: boolean = eq v0, v0
    v6: uint8 = sub v0, v0
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        for (value, zero, one) in [(4, 0, 1), (5, 0, 1), (6, 255, 0)] {
            let actual = KnownBits::analyse(
                Value(value),
                0,
                function,
                &definitions,
                TargetLayout::default(),
                &program.tree,
            );

            assert_eq!(actual, KnownBits { zero, one }, "v{value}");
        }
    }

    /// Bound leading zeros, trailing zeros, and population count from known bits.
    #[test]
    fn test_bound_bit_counts() {
        let program = TestModule::new(
            r#"
function test(v0: uint8): void {
entry(v0: uint8):
    v1: uint8 = 15
    v2: uint8 = 128
    v3: uint8 = and v0, v1
    v4: uint8 = or v3, v2
    v5: uint8 = intrinsic.math.bits.leadingZeroCount(v3)
    v6: uint8 = intrinsic.math.bits.trailingZeroCount(v4)
    v7: uint8 = intrinsic.math.bits.populationCount(v4)
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        for (value, zero, one) in [(5, 240, 0), (6, 248, 0), (7, 248, 0)] {
            let actual = KnownBits::analyse(
                Value(value),
                0,
                function,
                &definitions,
                TargetLayout::default(),
                &program.tree,
            );

            assert_eq!(actual, KnownBits { zero, one }, "v{value}");
        }
    }

    /// Move known bits through bit reversal and rotation.
    #[test]
    fn test_permute_known_bits() {
        let program = TestModule::new(
            r#"
function test(v0: uint8): void {
entry(v0: uint8):
    v1: uint8 = 15
    v2: uint8 = 128
    v3: uint8 = 1
    v4: uint8 = and v0, v1
    v5: uint8 = or v4, v2
    v6: uint8 = intrinsic.math.bits.bitReverse(v5)
    v7: uint8 = intrinsic.math.bits.rotateLeft(v5, v3)
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        for (value, zero, one) in [(6, 14, 1), (7, 224, 1)] {
            let actual = KnownBits::analyse(
                Value(value),
                0,
                function,
                &definitions,
                TargetLayout::default(),
                &program.tree,
            );

            assert_eq!(actual, KnownBits { zero, one }, "v{value}");
        }
    }

    /// Exclude impossible isolated bits from a partially known integer.
    #[test]
    fn test_bound_lowest_set_bit() {
        let program = TestModule::new(
            r#"
function test(v0: uint8): void {
entry(v0: uint8):
    v1: uint8 = 15
    v2: uint8 = 128
    v3: uint8 = and v0, v1
    v4: uint8 = or v3, v2
    v5: uint8 = intrinsic.math.bits.isolateLowestOne(v4)
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        let actual = KnownBits::analyse(
            Value(5),
            0,
            function,
            &definitions,
            TargetLayout::default(),
            &program.tree,
        );

        assert_eq!(actual, KnownBits { zero: 112, one: 0 });
    }

    /// Keep floating comparisons unknown when an operand may be NaN.
    #[test]
    fn test_preserve_float_comparisons() {
        let program = TestModule::new(
            r#"
function test(v0: float64): boolean {
entry(v0: float64):
    v1: boolean = eq v0, v0
    return v1
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        let bits = KnownBits::analyse(
            Value(1),
            0,
            function,
            &definitions,
            TargetLayout::default(),
            &program.tree,
        );

        assert_eq!((bits.zero, bits.one), (0, 0));
    }

    /// Preserve nonzero selections whose alternatives have different set bits.
    #[test]
    fn test_track_nonzero_selections() {
        let program = TestModule::new(
            r#"
function test(v0: boolean): uint8 {
entry(v0: boolean):
    v1: uint8 = 1
    v2: uint8 = 2
    v3: uint8 = select v0, v1, v2
    v4: uint8 = intrinsic.math.bits.bitReverse(v3)
    v5: uint8 = intrinsic.math.bits.populationCount(v3)
    v6: uint8 = intrinsic.math.bits.leadingZeroCount(v3)
    v7: uint8 = intrinsic.math.bits.trailingZeroCount(v3)
    v8: uint8 = intrinsic.math.arithmetic.unchecked.multiply(v3, v2)
    jump loop(v4)

loop(v9: uint8):
    branch v0 => loop(v9) | done

done:
    return v9
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        for (index, expected) in [
            (3, true),
            (4, true),
            (9, true),
            (5, true),
            (6, true),
            (7, false),
            (8, true),
        ] {
            let actual = Value(index).is_known_nonzero(
                function,
                &definitions,
                TargetLayout::default(),
                &program.tree,
            );

            assert_eq!(actual, expected, "v{index}");
        }
    }

    /// Bound the midpoint of a negative interval and its lower endpoint.
    #[test]
    fn test_bound_signed_midpoint() {
        let program = TestModule::new(
            r#"
function test(v0: int8): void {
entry(v0: int8):
    v1: int8 = 7
    v2: int8 = -8
    v3: int8 = and v0, v1
    v4: int8 = add v3, v2
    v5: int8 = intrinsic.math.arithmetic.midpoint(v4, v2)
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        let actual = KnownBits::analyse(
            Value(5),
            0,
            function,
            &definitions,
            TargetLayout::default(),
            &program.tree,
        );

        assert_eq!(actual, KnownBits { zero: 0, one: 248 });
    }

    /// Bound signed division rounded up and its nonnegative Euclidean remainder.
    #[test]
    fn test_bound_signed_quotients_and_euclidean_remainders() {
        let program = TestModule::new(
            r#"
function test(v0: int8): void {
entry(v0: int8):
    v1: int8 = 7
    v2: int8 = -8
    v3: int8 = 3
    v4: int8 = and v0, v1
    v5: int8 = add v4, v2
    v6: int8 = intrinsic.math.arithmetic.divideCeil(v5, v3)
    v7: int8 = intrinsic.math.arithmetic.remainderEuclidean(v5, v3)
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        for (value, zero, one) in [(6, 0, 0), (7, 252, 0)] {
            let actual = KnownBits::analyse(
                Value(value),
                0,
                function,
                &definitions,
                TargetLayout::default(),
                &program.tree,
            );

            assert_eq!(actual, KnownBits { zero, one }, "v{value}");
        }
    }

    /// Bound the unsigned distance between a negative interval and a positive value.
    #[test]
    fn test_bound_distance_across_zero() {
        let program = TestModule::new(
            r#"
function test(v0: int8): void {
entry(v0: int8):
    v1: int8 = 7
    v2: int8 = -8
    v3: int8 = and v0, v1
    v4: int8 = add v3, v2
    v5: uint8 = intrinsic.math.arithmetic.absDiff(v4, v1)
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        let actual = KnownBits::analyse(
            Value(5),
            0,
            function,
            &definitions,
            TargetLayout::default(),
            &program.tree,
        );

        assert_eq!(actual, KnownBits { zero: 240, one: 8 });
    }

    /// Set every bit to zero when a negative interval saturates to an unsigned type.
    #[test]
    fn test_saturate_negative_values_to_unsigned_zero() {
        let program = TestModule::new(
            r#"
function test(v0: int8): void {
entry(v0: int8):
    v1: int8 = 7
    v2: int8 = -8
    v3: int8 = and v0, v1
    v4: int8 = add v3, v2
    v5: uint8 = cast.intToIntSaturating v4 -> uint8
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        let actual = KnownBits::analyse(
            Value(5),
            0,
            function,
            &definitions,
            TargetLayout::default(),
            &program.tree,
        );

        assert_eq!(actual, KnownBits { zero: 255, one: 0 });
    }

    /// Keep bits unknown when a clamped interval spans negative and positive values.
    #[test]
    fn test_bound_clamped_signed_values() {
        let program = TestModule::new(
            r#"
function test(v0: int8): void {
entry(v0: int8):
    v1: int8 = 7
    v2: int8 = -8
    v3: int8 = intrinsic.math.arithmetic.clamp(v0, v2, v1)
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        let actual = KnownBits::analyse(
            Value(3),
            0,
            function,
            &definitions,
            TargetLayout::default(),
            &program.tree,
        );

        assert_eq!(actual, KnownBits { zero: 0, one: 0 });
    }

    /// Resolve division and right shifts that isolate the high unsigned bit.
    #[test]
    fn test_bound_high_unsigned_quotients() {
        let program = TestModule::new(
            r#"
function test(v0: uint64): void {
entry(v0: uint64):
    v1: uint64 = 9223372036854775808
    v2: uint64 = 63
    v3: uint64 = or v0, v1
    v4: uint64 = shr v3, v2
    v5: uint64 = div v3, v1
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        for (value, zero, one) in [
            (4, u128::from(u64::MAX) ^ 1, 1),
            (5, u128::from(u64::MAX) ^ 1, 1),
        ] {
            let actual = KnownBits::analyse(
                Value(value),
                0,
                function,
                &definitions,
                TargetLayout::default(),
                &program.tree,
            );

            assert_eq!(actual, KnownBits { zero, one }, "v{value}");
        }
    }

    /// Distinguish arithmetic sign extension from an unsigned shift at bit 127.
    #[test]
    fn test_extend_signed_and_unsigned_shift_results() {
        let program = TestModule::new(
            r#"
function test(v0: int128): void {
entry(v0: int128):
    v1: int128 = -170141183460469231731687303715884105728
    v2: int128 = 127
    v3: int128 = or v0, v1
    v4: int128 = shr v3, v2
    v5: uint128 = cast.bit v3 -> uint128
    v6: uint128 = 127
    v7: uint128 = shr v5, v6
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        for (value, zero, one) in [(4, 0, u128::MAX), (7, u128::MAX ^ 1, 1)] {
            let actual = KnownBits::analyse(
                Value(value),
                0,
                function,
                &definitions,
                TargetLayout::default(),
                &program.tree,
            );

            assert_eq!(actual, KnownBits { zero, one }, "v{value}");
        }
    }

    /// Wrap ordinary division of the signed minimum by minus one.
    #[test]
    fn test_wrap_signed_minimum_division() {
        let program = TestModule::new(
            r#"
function test(): void {
entry:
    v0: int128 = -170141183460469231731687303715884105728
    v1: int128 = -1
    v2: int128 = div v0, v1
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        let sign = 1u128 << 127;
        let actual = KnownBits::analyse(
            Value(2),
            0,
            function,
            &definitions,
            TargetLayout::default(),
            &program.tree,
        );

        assert_eq!(
            actual,
            KnownBits {
                zero: sign - 1,
                one: sign
            }
        );
    }

    /// Round the midpoint of opposite signed limits toward zero.
    #[test]
    fn test_round_opposite_signed_midpoint_toward_zero() {
        let program = TestModule::new(
            r#"
function test(): void {
entry:
    v0: int128 = 170141183460469231731687303715884105727
    v1: int128 = -170141183460469231731687303715884105728
    v2: int128 = intrinsic.math.arithmetic.midpoint(v0, v1)
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        let actual = KnownBits::analyse(
            Value(2),
            0,
            function,
            &definitions,
            TargetLayout::default(),
            &program.tree,
        );

        assert_eq!(
            actual,
            KnownBits {
                zero: u128::MAX,
                one: 0
            }
        );
    }

    /// Clamp subtraction below the signed minimum to that minimum.
    #[test]
    fn test_saturate_subtraction_at_signed_minimum() {
        let program = TestModule::new(
            r#"
function test(): void {
entry:
    v0: int128 = 170141183460469231731687303715884105727
    v1: int128 = -170141183460469231731687303715884105728
    v2: int128 = intrinsic.math.arithmetic.saturating.subtract(v1, v0)
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        let sign = 1u128 << 127;
        let actual = KnownBits::analyse(
            Value(2),
            0,
            function,
            &definitions,
            TargetLayout::default(),
            &program.tree,
        );

        assert_eq!(
            actual,
            KnownBits {
                zero: sign - 1,
                one: sign
            }
        );
    }

    /// Preserve negative bounds through signed narrowing, widening, and unsigned saturation.
    #[test]
    fn test_preserve_negative_conversion_bounds() {
        let program = TestModule::new(
            r#"
function test(v0: int128): void {
entry(v0: int128):
    v1: int128 = -170141183460469231731687303715884105728
    v2: int128 = or v0, v1
    v3: int64 = cast.intToIntSaturating v2 -> int64
    v4: uint128 = cast.intToIntSaturating v2 -> uint128
    v5: int128 = cast.intToInt v3 -> int128
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        for (value, zero, one) in [(3, 0, 1 << 63), (4, u128::MAX, 0), (5, 0, u128::MAX << 63)] {
            let actual = KnownBits::analyse(
                Value(value),
                0,
                function,
                &definitions,
                TargetLayout::default(),
                &program.tree,
            );

            assert_eq!(actual, KnownBits { zero, one }, "v{value}");
        }
    }

    /// Include the largest valid quotient when the dividend interval contains the signed minimum.
    #[test]
    fn test_bound_negative_division_by_minus_one() {
        let program = TestModule::new(
            r#"
function test(v0: int128): void {
entry(v0: int128):
    v1: int128 = -170141183460469231731687303715884105728
    v2: int128 = -1
    v3: int128 = or v0, v1
    v4: int128 = intrinsic.math.arithmetic.divideCeil(v3, v2)
    v5: int128 = 170141183460469231731687303715884105727
    v6: int128 = 1
    v7: int128 = and v0, v5
    v8: int128 = or v7, v6
    v9: int128 = intrinsic.math.arithmetic.divideCeil(v8, v2)
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let definitions = DefinitionTable::analyse(function, &program.tree);
        let sign = 1u128 << 127;
        for (value, expected) in [
            (Value(4), KnownBits { zero: sign, one: 0 }),
            (Value(9), KnownBits { zero: 0, one: sign }),
        ] {
            let actual = KnownBits::analyse(
                value,
                0,
                function,
                &definitions,
                TargetLayout::default(),
                &program.tree,
            );

            assert_eq!(actual, expected, "{value:?}");
        }
    }
}

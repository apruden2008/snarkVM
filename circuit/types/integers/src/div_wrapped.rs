// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkVM library.

// The snarkVM library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkVM library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkVM library. If not, see <https://www.gnu.org/licenses/>.

use super::*;

impl<E: Environment, I: IntegerType> DivWrapped<Self> for Integer<E, I> {
    type Output = Self;

    #[inline]
    fn div_wrapped(&self, other: &Integer<E, I>) -> Self::Output {
        match (self.is_constant(), other.is_constant()) {
            // If `other` is a constant and is zero, then halt.
            (_, true) if other.eject_value().is_zero() => E::halt("Attempted to divide by zero."),
            // If `self` and `other` are constants, and other is not zero, then directly return the value of the division.
            (true, true) => witness!(|self, other| console::Integer::new(self.wrapping_div(&other))),
            // Handle the remaining cases.
            // Note that `other` is either a constant and non-zero, or not a constant.
            _ => {
                if I::is_signed() {
                    // Divide the absolute value of `self` and `other` in the base field.
                    let unsigned_dividend = self.abs_wrapped().cast_as_dual();
                    // Note that `unsigned_divisor` is zero iff `other` is zero.
                    let unsigned_divisor = other.abs_wrapped().cast_as_dual();
                    // Note that this call to `div_wrapped` checks that `unsigned_divisor` is not zero.
                    let unsigned_quotient = unsigned_dividend.div_wrapped(&unsigned_divisor);

                    //  Note that quotient <= |console::Integer::MIN|, since the dividend <= |console::Integer::MIN| and 0 <= quotient <= dividend.
                    let signed_quotient = Self { bits_le: unsigned_quotient.bits_le, phantom: Default::default() };
                    let operands_same_sign = &self.msb().is_equal(other.msb());

                    // Note that this expression handles the wrapping case, where the dividend is `I::MIN` and the divisor is `-1` and the result should be `I::MIN`.
                    Self::ternary(operands_same_sign, &signed_quotient, &Self::zero().sub_wrapped(&signed_quotient))
                } else {
                    self.unsigned_division_via_witness(other).0
                }
            }
        }
    }
}

impl<E: Environment, I: IntegerType> Integer<E, I> {
    /// Divides `self` by `other`, via witnesses, returning the quotient and remainder.
    /// This method does not check that `other` is non-zero.
    /// This method should only be used when 2 * I::BITS < E::BaseField::size_in_data_bits().
    /// This method assumes the `self` and `other` are unsigned integers.
    pub(super) fn unsigned_division_via_witness(&self, other: &Self) -> (Self, Self) {
        // Eject the dividend and divisor, to compute the quotient as a witness.
        let dividend_value = self.eject_value();
        // Note: This band-aid was added to prevent a panic when the divisor is 0.
        let divisor_value = match other.eject_value().is_zero() {
            true => console::Integer::one(),
            false => other.eject_value(),
        };

        // Overflow is not possible for unsigned integers so we use wrapping operations.
        let quotient = Integer::new(Mode::Private, console::Integer::new(dividend_value.wrapping_div(&divisor_value)));
        let remainder = Integer::new(Mode::Private, console::Integer::new(dividend_value.wrapping_rem(&divisor_value)));

        if 2 * I::BITS < E::BaseField::size_in_data_bits() as u64 {
            // Ensure that Euclidean division holds for these values in the base field.
            E::assert_eq(self.to_field(), quotient.to_field() * other.to_field() + remainder.to_field());
        } else {
            // Ensure that Euclidean division holds for these values as integers.
            E::assert_eq(self, quotient.mul_checked(other).add_checked(&remainder));
        }

        // Ensure that the remainder is less than the divisor.
        // Note that if this check is satisfied and `other` is an unsigned integer, then `other` is not zero.
        E::assert(remainder.is_less_than(other));

        // Return the quotient and remainder of `self` and `other`.
        (quotient, remainder)
    }
}

impl<E: Environment, I: IntegerType> Metrics<dyn DivWrapped<Integer<E, I>, Output = Integer<E, I>>> for Integer<E, I> {
    type Case = (Mode, Mode);

    fn count(case: &Self::Case) -> Count {
        match (case.0, case.1) {
            (Mode::Constant, Mode::Constant) => Count::is(I::BITS, 0, 0, 0),
            (Mode::Constant, _) | (_, Mode::Constant) => {
                match (I::is_signed(), 2 * I::BITS < E::BaseField::size_in_data_bits() as u64) {
                    (true, true) => Count::less_than(5 * I::BITS + 1, 0, (9 * I::BITS) + 6, (9 * I::BITS) + 12),
                    (true, false) => Count::less_than(5 * I::BITS + 1, 0, 1611, 1814),
                    (false, true) => Count::less_than(I::BITS + 1, 0, (3 * I::BITS) + 2, (3 * I::BITS) + 5),
                    (false, false) => Count::less_than(I::BITS + 1, 0, 839, 1039),
                }
            }
            (_, _) => match (I::is_signed(), 2 * I::BITS < E::BaseField::size_in_data_bits() as u64) {
                (true, true) => Count::is(4 * I::BITS, 0, (9 * I::BITS) + 6, (9 * I::BITS) + 12),
                (true, false) => Count::is(4 * I::BITS, 0, 1611, 1814),
                (false, true) => Count::is(I::BITS, 0, (3 * I::BITS) + 2, (3 * I::BITS) + 5),
                (false, false) => Count::is(I::BITS, 0, 839, 1039),
            },
        }
    }
}

impl<E: Environment, I: IntegerType> OutputMode<dyn DivWrapped<Integer<E, I>, Output = Integer<E, I>>>
    for Integer<E, I>
{
    type Case = (Mode, Mode);

    fn output_mode(case: &Self::Case) -> Mode {
        match (case.0, case.1) {
            (Mode::Constant, Mode::Constant) => Mode::Constant,
            (_, _) => Mode::Private,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkvm_circuit_environment::Circuit;

    use test_utilities::*;

    use core::{ops::RangeInclusive, panic::RefUnwindSafe};

    const ITERATIONS: u64 = 32;

    fn check_div<I: IntegerType + RefUnwindSafe>(
        name: &str,
        first: console::Integer<<Circuit as Environment>::Network, I>,
        second: console::Integer<<Circuit as Environment>::Network, I>,
        mode_a: Mode,
        mode_b: Mode,
    ) {
        let a = Integer::<Circuit, I>::new(mode_a, first);
        let b = Integer::<Circuit, I>::new(mode_b, second);
        if second == console::Integer::zero() {
            match mode_b {
                Mode::Constant => check_operation_halts(&a, &b, Integer::div_wrapped),
                _ => Circuit::scope(name, || {
                    let _candidate = a.div_wrapped(&b);
                    assert_count_fails!(DivWrapped(Integer<I>, Integer<I>) => Integer<I>, &(mode_a, mode_b));
                }),
            }
        } else {
            let expected = first.wrapping_div(&second);
            Circuit::scope(name, || {
                let candidate = a.div_wrapped(&b);
                assert_eq!(expected, *candidate.eject_value());
                assert_eq!(console::Integer::new(expected), candidate.eject_value());
                assert_count!(DivWrapped(Integer<I>, Integer<I>) => Integer<I>, &(mode_a, mode_b));
                assert_output_mode!(DivWrapped(Integer<I>, Integer<I>) => Integer<I>, &(mode_a, mode_b), candidate);
            })
        }
        Circuit::reset();
    }

    fn run_test<I: IntegerType + RefUnwindSafe>(mode_a: Mode, mode_b: Mode) {
        let mut rng = TestRng::default();

        for _ in 0..ITERATIONS {
            let first = Uniform::rand(&mut rng);
            let second = Uniform::rand(&mut rng);

            let name = format!("Div: {first} / {second}");
            check_div::<I>(&name, first, second, mode_a, mode_b);

            let name = format!("Div by One: {first} / 1");
            check_div::<I>(&name, first, console::Integer::one(), mode_a, mode_b);

            let name = format!("Div by Self: {first} / {first}");
            check_div::<I>(&name, first, first, mode_a, mode_b);

            let name = format!("Div by Zero: {first} / 0");
            check_div::<I>(&name, first, console::Integer::zero(), mode_a, mode_b);
        }

        // Check standard division properties and corner cases.
        check_div::<I>("MAX / 1", console::Integer::MAX, console::Integer::one(), mode_a, mode_b);
        check_div::<I>("MIN / 1", console::Integer::MIN, console::Integer::one(), mode_a, mode_b);
        check_div::<I>("1 / 1", console::Integer::one(), console::Integer::one(), mode_a, mode_b);
        check_div::<I>("0 / 1", console::Integer::zero(), console::Integer::one(), mode_a, mode_b);
        check_div::<I>("MAX / 0", console::Integer::MAX, console::Integer::zero(), mode_a, mode_b);
        check_div::<I>("MIN / 0", console::Integer::MIN, console::Integer::zero(), mode_a, mode_b);
        check_div::<I>("1 / 0", console::Integer::one(), console::Integer::zero(), mode_a, mode_b);
        check_div::<I>("0 / 0", console::Integer::zero(), console::Integer::zero(), mode_a, mode_b);

        // Check some additional corner cases for signed integer division.
        if I::is_signed() {
            check_div::<I>("MAX / -1", console::Integer::MAX, -console::Integer::one(), mode_a, mode_b);
            check_div::<I>("MIN / -1", console::Integer::MIN, -console::Integer::one(), mode_a, mode_b);
            check_div::<I>("1 / -1", console::Integer::one(), -console::Integer::one(), mode_a, mode_b);
        }
    }

    fn run_exhaustive_test<I: IntegerType + RefUnwindSafe>(mode_a: Mode, mode_b: Mode)
    where
        RangeInclusive<I>: Iterator<Item = I>,
    {
        for first in I::MIN..=I::MAX {
            for second in I::MIN..=I::MAX {
                let first = console::Integer::<_, I>::new(first);
                let second = console::Integer::<_, I>::new(second);

                let name = format!("Div: ({first} / {second})");
                check_div::<I>(&name, first, second, mode_a, mode_b);
            }
        }
    }

    test_integer_binary!(run_test, i8, div);
    test_integer_binary!(run_test, i16, div);
    test_integer_binary!(run_test, i32, div);
    test_integer_binary!(run_test, i64, div);
    test_integer_binary!(run_test, i128, div);

    test_integer_binary!(run_test, u8, div);
    test_integer_binary!(run_test, u16, div);
    test_integer_binary!(run_test, u32, div);
    test_integer_binary!(run_test, u64, div);
    test_integer_binary!(run_test, u128, div);

    test_integer_binary!(#[ignore], run_exhaustive_test, u8, div, exhaustive);
    test_integer_binary!(#[ignore], run_exhaustive_test, i8, div, exhaustive);
}

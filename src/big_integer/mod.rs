//! A module for big-integer operations.
//!
//!
mod chip;
mod instructions;
use std::marker::PhantomData;

pub use chip::*;
pub use instructions::*;

use halo2wrong::halo2::{arithmetic::FieldExt, circuit::Value};
use maingate::{fe_to_big, AssignedValue};
use num_bigint::BigUint;

/// Trait for types representing a range of the limb.
pub trait RangeType: Clone {}

/// [`RangeType`] assigned to [`AssignedLimb`] and [`AssignedInteger`] that are not multiplied yet.
///
/// The maximum value of the [`Fresh`] type limb is defined in the chip implementing [`BigIntInstructions`] trait.
/// For example, [`BigIntChip`] has an `limb_width` parameter and limits the size of the [`Fresh`] type limb to be less than `2^(limb_width)`.
#[derive(Debug, Clone)]
pub struct Fresh {}
impl RangeType for Fresh {}

/// [`RangeType`] assigned to [`AssignedLimb`] and [`AssignedInteger`] that are already multiplied.
///
/// The size of the [`Muled`] type limb may overflow that of the [`Fresh`] type limb.
/// For this reason, we distinguish between these two types of integers.
#[derive(Debug, Clone)]
pub struct Muled {}
impl RangeType for Muled {}

/// An assigned limb of an non native integer.
#[derive(Debug, Clone)]
pub struct AssignedLimb<F: FieldExt, T: RangeType>(AssignedValue<F>, PhantomData<T>);

impl<F: FieldExt, T: RangeType> From<AssignedLimb<F, T>> for AssignedValue<F> {
    /// [`AssignedLimb`] can be also represented as [`AssignedValue`].
    fn from(limb: AssignedLimb<F, T>) -> Self {
        limb.0
    }
}

impl<F: FieldExt, T: RangeType> From<&AssignedLimb<F, T>> for AssignedValue<F> {
    /// The reference of [`AssignedLimb`] can be also represented as [`AssignedValue`].
    fn from(limb: &AssignedLimb<F, T>) -> Self {
        limb.0.clone()
    }
}

impl<F: FieldExt, T: RangeType> AssignedLimb<F, T> {
    /// Constructs new [`AssignedLimb`] from an assigned value.
    pub fn from(value: AssignedValue<F>) -> Self {
        AssignedLimb::<_, T>(value, PhantomData)
    }

    /// Returns the witness value as [`Value<Limb<F>>`].
    pub fn limb(&self) -> Value<Limb<F>> {
        self.0.value().map(|value| Limb::new(*value))
    }

    /// Returns the witness value as [`Value<F>`].
    pub fn value(&self) -> Value<F> {
        self.0.value().cloned()
    }

    /// Returns the witness value as [`Value<BigUint>`].
    pub fn to_big_uint(&self) -> Value<BigUint> {
        self.value().map(|f| fe_to_big(f))
    }

    /// Converts the [`RangeType`] from [`Fresh`] to [`Muled`].
    pub fn to_muled(self) -> AssignedLimb<F, Muled> {
        AssignedLimb::<F, Muled>(self.0, PhantomData)
    }
}

/// Limb that is about to be assigned.
#[derive(Debug, Clone)]
pub struct Limb<F: FieldExt>(F);

impl<F: FieldExt> Limb<F> {
    /// Creates a new [`Limb`]
    pub fn new(value: F) -> Self {
        Self(value)
    }
}

/// Witness integer that is about to be assigned.
#[derive(Debug, Clone)]
pub struct UnassignedInteger<F: FieldExt> {
    value: Value<Vec<F>>,
    num_limbs: usize,
}

impl<'a, F: FieldExt> From<Vec<F>> for UnassignedInteger<F> {
    /// Constructs new [`UnassignedInteger`] from a vector of witness values.
    fn from(value: Vec<F>) -> Self {
        let num_limbs = value.len();
        UnassignedInteger {
            value: Value::known(value),
            num_limbs,
        }
    }
}

impl<F: FieldExt> UnassignedInteger<F> {
    /// Returns indexed limb as [`Value`].
    fn limb(&self, idx: usize) -> Value<F> {
        self.value.as_ref().map(|e| e[idx])
    }

    /// Returns the number of the limbs.
    fn num_limbs(&self) -> usize {
        self.num_limbs
    }
}

/// An assigned witness integer.
#[derive(Debug, Clone)]
pub struct AssignedInteger<F: FieldExt, T: RangeType>(Vec<AssignedLimb<F, T>>);

impl<F: FieldExt, T: RangeType> AssignedInteger<F, T> {
    /// Creates a new [`AssignedInteger`].
    pub fn new(limbs: &[AssignedLimb<F, T>]) -> Self {
        AssignedInteger(limbs.to_vec())
    }

    /// Returns assigned limbs.
    pub fn limbs(&self) -> Vec<AssignedLimb<F, T>> {
        self.0.clone()
    }

    /// Returns indexed limb as [`Value`].
    fn limb(&self, idx: usize) -> AssignedValue<F> {
        self.0[idx].clone().into()
    }

    /// Returns the number of the limbs.
    pub fn num_limbs(&self) -> usize {
        self.0.len()
    }

    /// Returns the witness value as [`Value<BigUint>`].
    pub fn to_big_uint(&self, width: usize) -> Value<BigUint> {
        let num_limbs = self.num_limbs();
        (1..num_limbs).fold(
            self.limb(0).value().map(|f| fe_to_big(f.clone())),
            |acc, i| {
                acc + self
                    .limb(i)
                    .value()
                    .map(|f| fe_to_big(f.clone()) << (width * i))
            },
        )
    }

    /// Increases the number of the limbs by adding the given [`AssignedValue<F>`] representing zero.
    pub fn extend_limbs(&mut self, num_extend_limbs: usize, zero_value: AssignedValue<F>) {
        let pre_num_limbs = self.num_limbs();
        for _ in 0..num_extend_limbs {
            self.0.push(AssignedLimb::from(zero_value.clone()));
        }
        assert_eq!(pre_num_limbs + num_extend_limbs, self.num_limbs());
    }
}

impl<F: FieldExt> AssignedInteger<F, Fresh> {
    /// Converts the [`RangeType`] from [`Fresh`] to [`Muled`].
    pub fn to_muled(&self, zero_limb: AssignedLimb<F, Muled>) -> AssignedInteger<F, Muled> {
        let num_limb = self.num_limbs();
        let mut limbs = self
            .limbs()
            .into_iter()
            .map(|limb| limb.to_muled())
            .collect::<Vec<AssignedLimb<F, Muled>>>();
        for _ in 0..(num_limb - 1) {
            limbs.push(zero_limb.clone())
        }
        AssignedInteger::<F, Muled>::new(&limbs[..])
    }
}

use ark_bn254::Fr;
use ark_ff::PrimeField;
use ark_r1cs_std::{
	bits::{boolean::Boolean, uint128::UInt128},
	fields::fp::FpVar,
	prelude::{AllocVar, EqGadget},
};
use ark_relations::{lc, ns};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError, Variable};
use ark_sponge::{
	constraints::CryptographicSpongeVar,
	poseidon::constraints::PoseidonSpongeVar,
};
use rust_decimal::Decimal;

use crate::poseidon::poseidon_parameters;
use crate::tree::HashBytes;

const BALANCE_SCALE: u32 = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitInputError {
	NegativeBalance(Decimal),
	BalanceConversionOverflow(Decimal),
}

impl core::fmt::Display for CircuitInputError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::NegativeBalance(value) => write!(f, "balance must be non-negative, got {value}"),
			Self::BalanceConversionOverflow(value) => {
				write!(f, "balance conversion overflow for {value}")
			}
		}
	}
}

impl std::error::Error for CircuitInputError {}

#[derive(Debug, Clone)]
pub struct MerkleNodeRelationCircuit {
	pub left_hash: HashBytes,
	pub right_hash: HashBytes,
	pub parent_hash: HashBytes,
	pub left_balance_scaled: u128,
	pub right_balance_scaled: u128,
	pub parent_balance_scaled: u128,
}

impl MerkleNodeRelationCircuit {
	pub fn from_scaled(
		left_hash: HashBytes,
		right_hash: HashBytes,
		parent_hash: HashBytes,
		left_balance_scaled: u128,
		right_balance_scaled: u128,
		parent_balance_scaled: u128,
	) -> Self {
		Self {
			left_hash,
			right_hash,
			parent_hash,
			left_balance_scaled,
			right_balance_scaled,
			parent_balance_scaled,
		}
	}

	pub fn from_decimals(
		left_hash: HashBytes,
		right_hash: HashBytes,
		parent_hash: HashBytes,
		left_balance: Decimal,
		right_balance: Decimal,
		parent_balance: Decimal,
	) -> Result<Self, CircuitInputError> {
		Ok(Self {
			left_hash,
			right_hash,
			parent_hash,
			left_balance_scaled: decimal_to_scaled_u128(left_balance)?,
			right_balance_scaled: decimal_to_scaled_u128(right_balance)?,
			parent_balance_scaled: decimal_to_scaled_u128(parent_balance)?,
		})
	}
}

impl ConstraintSynthesizer<Fr> for MerkleNodeRelationCircuit {
	fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
		let left_hash_var = FpVar::<Fr>::new_witness(ns!(cs, "left_hash"), || {
			Ok(hash_bytes_to_field(&self.left_hash))
		})?;
		let right_hash_var = FpVar::<Fr>::new_witness(ns!(cs, "right_hash"), || {
			Ok(hash_bytes_to_field(&self.right_hash))
		})?;
		let parent_hash_var = FpVar::<Fr>::new_witness(ns!(cs, "parent_hash"), || {
			Ok(hash_bytes_to_field(&self.parent_hash))
		})?;

		let left_balance_var = UInt128::<Fr>::new_witness(ns!(cs, "left_balance"), || {
			Ok(self.left_balance_scaled)
		})?;
		let right_balance_var = UInt128::<Fr>::new_witness(ns!(cs, "right_balance"), || {
			Ok(self.right_balance_scaled)
		})?;
		let parent_balance_var = UInt128::<Fr>::new_witness(ns!(cs, "parent_balance"), || {
			Ok(self.parent_balance_scaled)
		})?;

		let left_balance_fp = FpVar::<Fr>::new_witness(ns!(cs, "left_balance_fp"), || {
			Ok(Fr::from(self.left_balance_scaled))
		})?;
		let right_balance_fp = FpVar::<Fr>::new_witness(ns!(cs, "right_balance_fp"), || {
			Ok(Fr::from(self.right_balance_scaled))
		})?;
		let parent_balance_fp = FpVar::<Fr>::new_witness(ns!(cs, "parent_balance_fp"), || {
			Ok(Fr::from(self.parent_balance_scaled))
		})?;

		// Sum constraint in field form.
		(left_balance_fp.clone() + right_balance_fp.clone()).enforce_equal(&parent_balance_fp)?;

		// Poseidon hash validity: parent_hash == Poseidon(left_hash, right_hash, left_balance, right_balance).
		let mut sponge = PoseidonSpongeVar::<Fr>::new(cs.clone(), poseidon_parameters());
		sponge.absorb(&left_hash_var)?;
		sponge.absorb(&right_hash_var)?;
		sponge.absorb(&left_balance_fp)?;
		sponge.absorb(&right_balance_fp)?;
		let expected_parent_hash = sponge.squeeze_field_elements(1)?[0].clone();
		expected_parent_hash.enforce_equal(&parent_hash_var)?;

		// Non-overflow over u128 addition via full-adder bit constraints with carry_128 == 0.
		enforce_u128_addition_no_overflow(
			cs,
			&left_balance_var,
			&right_balance_var,
			&parent_balance_var,
			self.left_balance_scaled,
			self.right_balance_scaled,
		)
	}
}

fn enforce_u128_addition_no_overflow(
	cs: ConstraintSystemRef<Fr>,
	left: &UInt128<Fr>,
	right: &UInt128<Fr>,
	parent: &UInt128<Fr>,
	left_value: u128,
	right_value: u128,
) -> Result<(), SynthesisError> {
	let left_bits = left.to_bits_le();
	let right_bits = right.to_bits_le();
	let parent_bits = parent.to_bits_le();

	let carry_values = compute_carry_bits(left_value, right_value);
	let mut carries = Vec::with_capacity(129);
	for carry_value in carry_values.iter() {
		let carry = Boolean::<Fr>::new_witness(cs.clone(), || Ok(*carry_value))?;
		carries.push(carry);
	}

	for (((left_bit, right_bit), parent_bit), carry_pair) in left_bits
		.iter()
		.zip(right_bits.iter())
		.zip(parent_bits.iter())
		.zip(carries.windows(2))
	{
		let li = Boolean::le_bits_to_fp_var(core::slice::from_ref(left_bit))?;
		let ri = Boolean::le_bits_to_fp_var(core::slice::from_ref(right_bit))?;
		let ci = Boolean::le_bits_to_fp_var(core::slice::from_ref(&carry_pair[0]))?;
		let pi = Boolean::le_bits_to_fp_var(core::slice::from_ref(parent_bit))?;
		let co = Boolean::le_bits_to_fp_var(core::slice::from_ref(&carry_pair[1]))?;

		(li + ri + ci).enforce_equal(&(pi + co.clone() + co))?;
	}

	// Final carry must be zero to guarantee no u128 overflow.
	cs.enforce_constraint(lc!() + Variable::One, carries[128].lc(), lc!())
}

fn compute_carry_bits(left: u128, right: u128) -> [bool; 129] {
	let mut carries = [false; 129];
	let mut carry = false;
	for (i, carry_slot) in carries.iter_mut().take(128).enumerate() {
		*carry_slot = carry;
		let a = ((left >> i) & 1) == 1;
		let b = ((right >> i) & 1) == 1;
		carry = (a & b) | (a & carry) | (b & carry);
	}
	carries[128] = carry;
	carries
}

fn hash_bytes_to_field(hash: &HashBytes) -> Fr {
	Fr::from_le_bytes_mod_order(hash)
}

fn decimal_to_scaled_u128(value: Decimal) -> Result<u128, CircuitInputError> {
	if value.is_sign_negative() {
		return Err(CircuitInputError::NegativeBalance(value));
	}

	let mut scaled = value;
	scaled.rescale(BALANCE_SCALE);

	let mantissa = scaled.mantissa();
	if mantissa < 0 {
		return Err(CircuitInputError::NegativeBalance(value));
	}

	u128::try_from(mantissa).map_err(|_| CircuitInputError::BalanceConversionOverflow(value))
}

#[cfg(test)]
mod tests {
	use super::MerkleNodeRelationCircuit;
	use crate::poseidon::poseidon_internal_hash;
	use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
	use rust_decimal_macros::dec;

	#[test]
	fn circuit_accepts_valid_merkle_parent_relation() {
		let left_hash = [11u8; 32];
		let right_hash = [22u8; 32];
		let left_balance = dec!(10.50000000);
		let right_balance = dec!(2.25000000);
		let parent_balance = dec!(12.75000000);
		let parent_hash =
			poseidon_internal_hash(&left_hash, &right_hash, &left_balance, &right_balance).expect("hash must succeed");

		let circuit = MerkleNodeRelationCircuit::from_decimals(
			left_hash,
			right_hash,
			parent_hash,
			left_balance,
			right_balance,
			parent_balance,
		)
		.expect("circuit input must be valid");

		let cs = ConstraintSystem::new_ref();
		circuit.generate_constraints(cs.clone()).expect("constraint generation must succeed");
		assert!(cs.is_satisfied().expect("cs evaluation must succeed"));
	}

	#[test]
	fn circuit_rejects_invalid_parent_hash() {
		let left_hash = [11u8; 32];
		let right_hash = [22u8; 32];

		let circuit = MerkleNodeRelationCircuit::from_decimals(
			left_hash,
			right_hash,
			[99u8; 32],
			dec!(10.5),
			dec!(2.25),
			dec!(12.75),
		)
		.expect("circuit input must be valid");

		let cs = ConstraintSystem::new_ref();
		circuit.generate_constraints(cs.clone()).expect("constraint generation must succeed");
		assert!(!cs.is_satisfied().expect("cs evaluation must succeed"));
	}

	#[test]
	fn circuit_rejects_overflowing_u128_addition() {
		let circuit = MerkleNodeRelationCircuit::from_scaled(
			[1u8; 32],
			[2u8; 32],
			[3u8; 32],
			u128::MAX,
			1,
			0,
		);

		let cs = ConstraintSystem::new_ref();
		circuit.generate_constraints(cs.clone()).expect("constraint generation must succeed");
		assert!(!cs.is_satisfied().expect("cs evaluation must succeed"));
	}
}

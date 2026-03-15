use ark_bn254::Fr;
use ark_ff::{BigInteger, One, PrimeField, UniformRand, Zero};
use ark_sponge::{poseidon::{PoseidonParameters, PoseidonSponge}, CryptographicSponge};
use rand::{rngs::StdRng, SeedableRng};
use rust_decimal::Decimal;

use crate::tree::HashBytes;

const BALANCE_SCALE: u32 = 8;
const FULL_ROUNDS: u64 = 8;
const PARTIAL_ROUNDS: u64 = 57;
const POSEIDON_ALPHA: u64 = 5;
const STATE_WIDTH: usize = 3;
const PARAM_SEED: [u8; 32] = [42u8; 32];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoseidonError {
    NegativeBalance(Decimal),
    BalanceOverflow(Decimal),
}

impl core::fmt::Display for PoseidonError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NegativeBalance(value) => write!(f, "balance must be non-negative, got {value}"),
            Self::BalanceOverflow(value) => write!(f, "balance conversion overflow for {value}"),
        }
    }
}

impl std::error::Error for PoseidonError {}

pub fn poseidon_leaf_hash(user_id: u64, balance: &Decimal) -> Result<HashBytes, PoseidonError> {
    let mut sponge = PoseidonSponge::<Fr>::new(poseidon_params());

    sponge.absorb(&Fr::from(user_id));
    sponge.absorb(&decimal_to_field(balance)?);

    Ok(field_to_hash_bytes(sponge.squeeze_field_elements(1)[0]))
}

pub fn poseidon_internal_hash(
    left_hash: &HashBytes,
    right_hash: &HashBytes,
    left_balance: &Decimal,
    right_balance: &Decimal,
) -> Result<HashBytes, PoseidonError> {
    let mut sponge = PoseidonSponge::<Fr>::new(poseidon_params());

    sponge.absorb(&hash_to_field(left_hash));
    sponge.absorb(&hash_to_field(right_hash));
    sponge.absorb(&decimal_to_field(left_balance)?);
    sponge.absorb(&decimal_to_field(right_balance)?);

    Ok(field_to_hash_bytes(sponge.squeeze_field_elements(1)[0]))
}

fn poseidon_params() -> &'static PoseidonParameters<Fr> {
    static PARAMS: std::sync::OnceLock<PoseidonParameters<Fr>> = std::sync::OnceLock::new();
    PARAMS.get_or_init(build_poseidon_parameters)
}

fn build_poseidon_parameters() -> PoseidonParameters<Fr> {
    let mut rng = StdRng::from_seed(PARAM_SEED);
    let total_rounds = (FULL_ROUNDS + PARTIAL_ROUNDS) as u32;

    let mds = build_deterministic_mds(&mut rng);
    let ark = PoseidonParameters::<Fr>::random_ark(total_rounds, &mut rng);

    PoseidonParameters::new(
        FULL_ROUNDS as u32,
        PARTIAL_ROUNDS as u32,
        POSEIDON_ALPHA,
        mds,
        ark,
    )
}

fn build_deterministic_mds(rng: &mut StdRng) -> Vec<Vec<Fr>> {
    let mut mds = vec![vec![Fr::zero(); STATE_WIDTH]; STATE_WIDTH];

    for (row_idx, row) in mds.iter_mut().enumerate() {
        for value in row.iter_mut() {
            *value = Fr::rand(rng);
        }
        row[row_idx] += Fr::one();
    }

    mds
}

fn decimal_to_field(value: &Decimal) -> Result<Fr, PoseidonError> {
    if value.is_sign_negative() {
        return Err(PoseidonError::NegativeBalance(*value));
    }

    let mut scaled = *value;
    scaled.rescale(BALANCE_SCALE);

    let mantissa = scaled.mantissa();
    if mantissa < 0 {
        return Err(PoseidonError::NegativeBalance(*value));
    }

    let as_u128 = u128::try_from(mantissa).map_err(|_| PoseidonError::BalanceOverflow(*value))?;
    Ok(Fr::from(as_u128))
}

fn hash_to_field(hash: &HashBytes) -> Fr {
    Fr::from_le_bytes_mod_order(hash)
}

fn field_to_hash_bytes(field: Fr) -> HashBytes {
    let bytes = field.into_repr().to_bytes_le();

    let mut out = [0u8; 32];
    let copy_len = core::cmp::min(out.len(), bytes.len());
    out[..copy_len].copy_from_slice(&bytes[..copy_len]);
    out
}

#[cfg(test)]
mod tests {
    use super::{poseidon_internal_hash, poseidon_leaf_hash, PoseidonError};
    use rust_decimal_macros::dec;

    #[test]
    fn poseidon_leaf_hash_is_deterministic() {
        let h1 = poseidon_leaf_hash(7, &dec!(12.34000000)).expect("hash must succeed");
        let h2 = poseidon_leaf_hash(7, &dec!(12.34000000)).expect("hash must succeed");
        assert_eq!(h1, h2);
    }

    #[test]
    fn poseidon_leaf_hash_changes_with_input() {
        let h1 = poseidon_leaf_hash(7, &dec!(12.34)).expect("hash must succeed");
        let h2 = poseidon_leaf_hash(8, &dec!(12.34)).expect("hash must succeed");
        let h3 = poseidon_leaf_hash(7, &dec!(12.35)).expect("hash must succeed");

        assert_ne!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn poseidon_internal_hash_uses_all_inputs() {
        let l1 = poseidon_leaf_hash(1, &dec!(10)).expect("hash must succeed");
        let r1 = poseidon_leaf_hash(2, &dec!(5)).expect("hash must succeed");

        let base = poseidon_internal_hash(&l1, &r1, &dec!(10), &dec!(5)).expect("hash must succeed");
        let changed_balance =
            poseidon_internal_hash(&l1, &r1, &dec!(11), &dec!(5)).expect("hash must succeed");
        let changed_child = poseidon_internal_hash(&r1, &l1, &dec!(10), &dec!(5)).expect("hash must succeed");

        assert_ne!(base, changed_balance);
        assert_ne!(base, changed_child);
    }

    #[test]
    fn poseidon_rejects_negative_balance() {
        let err = poseidon_leaf_hash(1, &dec!(-1.0)).expect_err("negative balance must fail");
        assert_eq!(err, PoseidonError::NegativeBalance(dec!(-1.0)));
    }
}

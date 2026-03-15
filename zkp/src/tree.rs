use rust_decimal::Decimal;

use crate::poseidon::{poseidon_leaf_hash, PoseidonError};

pub const HASH_BYTES: usize = 32;
pub type HashBytes = [u8; HASH_BYTES];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerkleNode {
	pub hash: HashBytes,
	pub balance: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BalanceSnapshot {
	pub user_id: u64,
	pub balance: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbBalanceSnapshot {
	pub user_id: i64,
	pub available: Decimal,
	pub locked: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeError {
	EmptySnapshotInput,
	InvalidUserId(i64),
	NegativeBalance { user_id: u64, balance: Decimal },
	BalanceOverflow { user_id: u64 },
	HashingError(PoseidonError),
}

impl core::fmt::Display for TreeError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::EmptySnapshotInput => write!(f, "snapshot input must not be empty"),
			Self::InvalidUserId(user_id) => write!(f, "invalid db user_id (must be >= 0): {user_id}"),
			Self::NegativeBalance { user_id, balance } => {
				write!(f, "negative balance for user {user_id}: {balance}")
			}
			Self::BalanceOverflow { user_id } => {
				write!(f, "balance overflow while aggregating snapshot for user {user_id}")
			}
			Self::HashingError(err) => write!(f, "poseidon hashing failed: {err}"),
		}
	}
}

impl std::error::Error for TreeError {}

impl From<PoseidonError> for TreeError {
	fn from(value: PoseidonError) -> Self {
		Self::HashingError(value)
	}
}

pub fn build_leaf_nodes<F>(
	snapshots: &[BalanceSnapshot],
	mut leaf_hasher: F,
) -> Result<Vec<MerkleNode>, TreeError>
where
	F: FnMut(u64, &Decimal) -> HashBytes,
{
	if snapshots.is_empty() {
		return Err(TreeError::EmptySnapshotInput);
	}

	let mut leaves = Vec::with_capacity(snapshots.len());
	for snapshot in snapshots {
		if snapshot.balance.is_sign_negative() {
			return Err(TreeError::NegativeBalance {
				user_id: snapshot.user_id,
				balance: snapshot.balance,
			});
		}

		leaves.push(MerkleNode {
			hash: leaf_hasher(snapshot.user_id, &snapshot.balance),
			balance: snapshot.balance,
		});
	}

	Ok(leaves)
}

pub fn build_leaf_nodes_from_db_snapshots<F>(
	db_snapshots: &[DbBalanceSnapshot],
	leaf_hasher: F,
) -> Result<Vec<MerkleNode>, TreeError>
where
	F: FnMut(u64, &Decimal) -> HashBytes,
{
	let normalized = normalize_db_snapshots(db_snapshots)?;
	build_leaf_nodes(&normalized, leaf_hasher)
}

pub fn build_poseidon_leaf_nodes(snapshots: &[BalanceSnapshot]) -> Result<Vec<MerkleNode>, TreeError> {
	if snapshots.is_empty() {
		return Err(TreeError::EmptySnapshotInput);
	}

	let mut leaves = Vec::with_capacity(snapshots.len());
	for snapshot in snapshots {
		if snapshot.balance.is_sign_negative() {
			return Err(TreeError::NegativeBalance {
				user_id: snapshot.user_id,
				balance: snapshot.balance,
			});
		}

		let hash = poseidon_leaf_hash(snapshot.user_id, &snapshot.balance)?;
		leaves.push(MerkleNode {
			hash,
			balance: snapshot.balance,
		});
	}

	Ok(leaves)
}

pub fn build_poseidon_leaf_nodes_from_db_snapshots(
	db_snapshots: &[DbBalanceSnapshot],
) -> Result<Vec<MerkleNode>, TreeError> {
	let normalized = normalize_db_snapshots(db_snapshots)?;
	build_poseidon_leaf_nodes(&normalized)
}

fn normalize_db_snapshots(db_snapshots: &[DbBalanceSnapshot]) -> Result<Vec<BalanceSnapshot>, TreeError> {
	if db_snapshots.is_empty() {
		return Err(TreeError::EmptySnapshotInput);
	}

	let mut normalized = Vec::with_capacity(db_snapshots.len());
	for row in db_snapshots {
		if row.user_id < 0 {
			return Err(TreeError::InvalidUserId(row.user_id));
		}

		let user_id = row.user_id as u64;
		let balance = row
			.available
			.checked_add(row.locked)
			.ok_or(TreeError::BalanceOverflow { user_id })?;

		normalized.push(BalanceSnapshot { user_id, balance });
	}

	Ok(normalized)
}

#[cfg(test)]
mod tests {
	use super::{
		build_leaf_nodes, build_leaf_nodes_from_db_snapshots, build_poseidon_leaf_nodes,
		BalanceSnapshot, DbBalanceSnapshot, TreeError,
	};
	use rust_decimal::Decimal;
	use rust_decimal_macros::dec;

	fn deterministic_hash(user_id: u64, balance: &Decimal) -> [u8; 32] {
		let mut hash = [0u8; 32];
		hash[0] = (user_id & 0xFF) as u8;
		hash[1] = (balance.scale() & 0xFF) as u8;
		hash
	}

	#[test]
	fn build_leaf_nodes_successfully_from_snapshots() {
		let snapshots = vec![
			BalanceSnapshot {
				user_id: 1,
				balance: dec!(12.5),
			},
			BalanceSnapshot {
				user_id: 2,
				balance: dec!(7.25),
			},
		];

		let leaves = build_leaf_nodes(&snapshots, deterministic_hash).expect("leaf build must succeed");
		assert_eq!(leaves.len(), 2);
		assert_eq!(leaves[0].balance, dec!(12.5));
		assert_eq!(leaves[0].hash[0], 1);
		assert_eq!(leaves[1].balance, dec!(7.25));
		assert_eq!(leaves[1].hash[0], 2);
	}

	#[test]
	fn build_leaf_nodes_from_db_snapshots_aggregates_balances() {
		let db_snapshots = vec![
			DbBalanceSnapshot {
				user_id: 42,
				available: dec!(10.50),
				locked: dec!(1.25),
			},
			DbBalanceSnapshot {
				user_id: 7,
				available: dec!(3),
				locked: dec!(0),
			},
		];

		let leaves =
			build_leaf_nodes_from_db_snapshots(&db_snapshots, deterministic_hash).expect("leaf build must succeed");

		assert_eq!(leaves.len(), 2);
		assert_eq!(leaves[0].balance, dec!(11.75));
		assert_eq!(leaves[0].hash[0], 42);
		assert_eq!(leaves[1].balance, dec!(3));
		assert_eq!(leaves[1].hash[0], 7);
	}

	#[test]
	fn build_leaf_nodes_rejects_negative_balance() {
		let snapshots = vec![BalanceSnapshot {
			user_id: 9,
			balance: dec!(-0.1),
		}];

		let err = build_leaf_nodes(&snapshots, deterministic_hash).expect_err("negative balance must fail");
		assert_eq!(
			err,
			TreeError::NegativeBalance {
				user_id: 9,
				balance: dec!(-0.1)
			}
		);
	}

	#[test]
	fn build_leaf_nodes_rejects_empty_input() {
		let err = build_leaf_nodes(&[], deterministic_hash).expect_err("empty input must fail");
		assert_eq!(err, TreeError::EmptySnapshotInput);
	}

	#[test]
	fn build_leaf_nodes_from_db_snapshots_rejects_negative_user_id() {
		let db_snapshots = vec![DbBalanceSnapshot {
			user_id: -1,
			available: dec!(1),
			locked: dec!(2),
		}];

		let err =
			build_leaf_nodes_from_db_snapshots(&db_snapshots, deterministic_hash).expect_err("invalid user id must fail");
		assert_eq!(err, TreeError::InvalidUserId(-1));
	}

	#[test]
	fn build_poseidon_leaf_nodes_successfully() {
		let snapshots = vec![
			BalanceSnapshot {
				user_id: 11,
				balance: dec!(100.01),
			},
			BalanceSnapshot {
				user_id: 12,
				balance: dec!(99.99),
			},
		];

		let leaves = build_poseidon_leaf_nodes(&snapshots).expect("poseidon leaf build must succeed");
		assert_eq!(leaves.len(), 2);
		assert_ne!(leaves[0].hash, leaves[1].hash);
	}
}

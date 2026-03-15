use rust_decimal::Decimal;

use crate::poseidon::{poseidon_internal_hash, poseidon_leaf_hash, PoseidonError};

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
pub struct MerkleProofStep {
	pub sibling_hash: HashBytes,
	pub sibling_balance: Decimal,
	pub sibling_is_left: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerkleProof {
	pub leaf_index: usize,
	pub leaf: MerkleNode,
	pub path: Vec<MerkleProofStep>,
	pub root: MerkleNode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerkleSumTree {
	levels: Vec<Vec<MerkleNode>>,
	original_leaf_count: usize,
}

impl MerkleSumTree {
	pub fn root(&self) -> &MerkleNode {
		&self.levels[self.levels.len() - 1][0]
	}

	pub fn levels(&self) -> &[Vec<MerkleNode>] {
		&self.levels
	}

	pub fn original_leaf_count(&self) -> usize {
		self.original_leaf_count
	}

	pub fn padded_leaf_count(&self) -> usize {
		self.levels[0].len()
	}

	pub fn generate_proof(&self, leaf_index: usize) -> Result<MerkleProof, TreeError> {
		if leaf_index >= self.original_leaf_count {
			return Err(TreeError::InvalidLeafIndex {
				index: leaf_index,
				leaf_count: self.original_leaf_count,
			});
		}

		let mut index = leaf_index;
		let mut path = Vec::with_capacity(self.levels.len().saturating_sub(1));

		for level_nodes in self.levels.iter().take(self.levels.len() - 1) {
			let is_right = index % 2 == 1;
			let sibling_index = if is_right { index - 1 } else { index + 1 };
			let sibling = level_nodes[sibling_index].clone();

			path.push(MerkleProofStep {
				sibling_hash: sibling.hash,
				sibling_balance: sibling.balance,
				sibling_is_left: is_right,
			});

			index /= 2;
		}

		Ok(MerkleProof {
			leaf_index,
			leaf: self.levels[0][leaf_index].clone(),
			path,
			root: self.root().clone(),
		})
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeError {
	EmptySnapshotInput,
	InvalidUserId(i64),
	InvalidLeafIndex { index: usize, leaf_count: usize },
	NegativeBalance { user_id: u64, balance: Decimal },
	BalanceOverflow { user_id: u64 },
	ParentBalanceOverflow,
	HashingError(PoseidonError),
}

impl core::fmt::Display for TreeError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::EmptySnapshotInput => write!(f, "snapshot input must not be empty"),
			Self::InvalidUserId(user_id) => write!(f, "invalid db user_id (must be >= 0): {user_id}"),
			Self::InvalidLeafIndex { index, leaf_count } => {
				write!(f, "invalid leaf index {index}, total leaves: {leaf_count}")
			}
			Self::NegativeBalance { user_id, balance } => {
				write!(f, "negative balance for user {user_id}: {balance}")
			}
			Self::BalanceOverflow { user_id } => {
				write!(f, "balance overflow while aggregating snapshot for user {user_id}")
			}
			Self::ParentBalanceOverflow => write!(f, "balance overflow while building parent node"),
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

pub fn build_merkle_sum_tree_from_leaves<F>(
	leaves: &[MerkleNode],
	mut internal_hasher: F,
) -> Result<MerkleSumTree, TreeError>
where
	F: FnMut(&MerkleNode, &MerkleNode) -> Result<HashBytes, TreeError>,
{
	if leaves.is_empty() {
		return Err(TreeError::EmptySnapshotInput);
	}

	let original_leaf_count = leaves.len();
	let mut levels: Vec<Vec<MerkleNode>> = Vec::new();
	let mut current_level = leaves.to_vec();

	if current_level.len() % 2 == 1 {
		current_level.push(current_level[current_level.len() - 1].clone());
	}
	levels.push(current_level.clone());

	while current_level.len() > 1 {
		let mut next_level = Vec::with_capacity(current_level.len() / 2);

		for pair in current_level.chunks_exact(2) {
			let left = &pair[0];
			let right = &pair[1];

			let parent_balance = left
				.balance
				.checked_add(right.balance)
				.ok_or(TreeError::ParentBalanceOverflow)?;
			let parent_hash = internal_hasher(left, right)?;

			next_level.push(MerkleNode {
				hash: parent_hash,
				balance: parent_balance,
			});
		}

		if next_level.len() > 1 && next_level.len() % 2 == 1 {
			next_level.push(next_level[next_level.len() - 1].clone());
		}

		levels.push(next_level.clone());
		current_level = next_level;
	}

	Ok(MerkleSumTree {
		levels,
		original_leaf_count,
	})
}

pub fn build_poseidon_merkle_sum_tree_from_leaves(leaves: &[MerkleNode]) -> Result<MerkleSumTree, TreeError> {
	build_merkle_sum_tree_from_leaves(leaves, |left, right| {
		poseidon_internal_hash(&left.hash, &right.hash, &left.balance, &right.balance).map_err(TreeError::from)
	})
}

pub fn build_poseidon_merkle_sum_tree(snapshots: &[BalanceSnapshot]) -> Result<MerkleSumTree, TreeError> {
	let leaves = build_poseidon_leaf_nodes(snapshots)?;
	build_poseidon_merkle_sum_tree_from_leaves(&leaves)
}

pub fn build_poseidon_merkle_sum_tree_from_db_snapshots(
	db_snapshots: &[DbBalanceSnapshot],
) -> Result<MerkleSumTree, TreeError> {
	let leaves = build_poseidon_leaf_nodes_from_db_snapshots(db_snapshots)?;
	build_poseidon_merkle_sum_tree_from_leaves(&leaves)
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
		build_leaf_nodes, build_leaf_nodes_from_db_snapshots, build_merkle_sum_tree_from_leaves,
		build_poseidon_leaf_nodes, build_poseidon_merkle_sum_tree, BalanceSnapshot, DbBalanceSnapshot,
		MerkleNode, TreeError,
	};
	use rust_decimal::Decimal;
	use rust_decimal_macros::dec;

	fn deterministic_hash(user_id: u64, balance: &Decimal) -> [u8; 32] {
		let mut hash = [0u8; 32];
		hash[0] = (user_id & 0xFF) as u8;
		hash[1] = (balance.scale() & 0xFF) as u8;
		hash
	}

	fn deterministic_internal_hash(left: &MerkleNode, right: &MerkleNode) -> Result<[u8; 32], TreeError> {
		let mut hash = [0u8; 32];
		hash[0] = left.hash[0].wrapping_add(right.hash[0]);
		hash[1] = ((left.balance.mantissa() + right.balance.mantissa()) & 0xFF) as u8;
		Ok(hash)
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

	#[test]
	fn build_merkle_sum_tree_bottom_up_with_padding() {
		let leaves = vec![
			MerkleNode {
				hash: [1u8; 32],
				balance: dec!(10),
			},
			MerkleNode {
				hash: [2u8; 32],
				balance: dec!(5),
			},
			MerkleNode {
				hash: [3u8; 32],
				balance: dec!(7),
			},
		];

		let tree =
			build_merkle_sum_tree_from_leaves(&leaves, deterministic_internal_hash).expect("tree build must succeed");

		assert_eq!(tree.original_leaf_count(), 3);
		assert_eq!(tree.padded_leaf_count(), 4);
		assert_eq!(tree.levels().len(), 3);
		assert_eq!(tree.levels()[0][2], tree.levels()[0][3]);
		assert_eq!(tree.root().balance, dec!(29));
	}

	#[test]
	fn generate_merkle_proof_returns_expected_path_depth() {
		let snapshots = vec![
			BalanceSnapshot {
				user_id: 1,
				balance: dec!(4),
			},
			BalanceSnapshot {
				user_id: 2,
				balance: dec!(6),
			},
			BalanceSnapshot {
				user_id: 3,
				balance: dec!(10),
			},
		];

		let tree = build_poseidon_merkle_sum_tree(&snapshots).expect("poseidon tree build must succeed");
		let proof = tree.generate_proof(1).expect("proof generation must succeed");

		assert_eq!(proof.leaf_index, 1);
		assert_eq!(proof.path.len(), tree.levels().len() - 1);
		assert_eq!(proof.root, tree.root().clone());
	}

	#[test]
	fn generate_merkle_proof_rejects_out_of_range_leaf_index() {
		let snapshots = vec![
			BalanceSnapshot {
				user_id: 1,
				balance: dec!(1),
			},
			BalanceSnapshot {
				user_id: 2,
				balance: dec!(2),
			},
		];

		let tree = build_poseidon_merkle_sum_tree(&snapshots).expect("poseidon tree build must succeed");
		let err = tree.generate_proof(2).expect_err("out-of-range index must fail");

		assert_eq!(
			err,
			TreeError::InvalidLeafIndex {
				index: 2,
				leaf_count: 2,
			}
		);
	}
}

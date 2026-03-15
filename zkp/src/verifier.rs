use rust_decimal::Decimal;
use serde::Deserialize;

use crate::poseidon::{poseidon_internal_hash, poseidon_leaf_hash};
use crate::tree::HashBytes;

#[derive(Debug, Deserialize)]
pub struct ProofStepPayload {
    pub sibling_hash: String,
    pub sibling_balance: String,
    pub sibling_is_left: bool,
}

#[derive(Debug, Deserialize)]
pub struct ProofPayload {
    pub user_id: u64,
    pub leaf_balance: String,
    pub root_hash: String,
    pub root_balance: String,
    pub merkle_path: Vec<ProofStepPayload>,
}

#[derive(Debug, Deserialize)]
pub struct PublicInputsPayload {
    pub expected_root_hash: String,
    pub expected_root_balance: String,
    pub expected_user_id: Option<u64>,
}

pub fn verify_proof_json(proof_json: &str, public_inputs_json: &str) -> bool {
    let proof: ProofPayload = match serde_json::from_str(proof_json) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let public_inputs: PublicInputsPayload = match serde_json::from_str(public_inputs_json) {
        Ok(v) => v,
        Err(_) => return false,
    };

    verify_proof_payload(&proof, &public_inputs).unwrap_or(false)
}

fn verify_proof_payload(
    proof: &ProofPayload,
    public_inputs: &PublicInputsPayload,
) -> Result<bool, String> {
    if let Some(expected_user_id) = public_inputs.expected_user_id {
        if expected_user_id != proof.user_id {
            return Ok(false);
        }
    }

    let leaf_balance = parse_decimal(&proof.leaf_balance)?;
    let declared_root_balance = parse_decimal(&proof.root_balance)?;
    let expected_root_balance = parse_decimal(&public_inputs.expected_root_balance)?;

    let mut current_hash = poseidon_leaf_hash(proof.user_id, &leaf_balance).map_err(|e| e.to_string())?;
    let mut current_balance = leaf_balance;

    for step in &proof.merkle_path {
        let sibling_hash = parse_hash_hex(&step.sibling_hash)?;
        let sibling_balance = parse_decimal(&step.sibling_balance)?;

        if sibling_balance.is_sign_negative() {
            return Ok(false);
        }

        if step.sibling_is_left {
            current_hash = poseidon_internal_hash(
                &sibling_hash,
                &current_hash,
                &sibling_balance,
                &current_balance,
            )
            .map_err(|e| e.to_string())?;
            current_balance = sibling_balance
                .checked_add(current_balance)
                .ok_or_else(|| "balance overflow while recomputing path".to_string())?;
        } else {
            current_hash = poseidon_internal_hash(
                &current_hash,
                &sibling_hash,
                &current_balance,
                &sibling_balance,
            )
            .map_err(|e| e.to_string())?;
            current_balance = current_balance
                .checked_add(sibling_balance)
                .ok_or_else(|| "balance overflow while recomputing path".to_string())?;
        }
    }

    let declared_root_hash = parse_hash_hex(&proof.root_hash)?;
    let expected_root_hash = parse_hash_hex(&public_inputs.expected_root_hash)?;

    Ok(
        current_hash == declared_root_hash
            && current_hash == expected_root_hash
            && current_balance == declared_root_balance
            && current_balance == expected_root_balance,
    )
}

fn parse_hash_hex(value: &str) -> Result<HashBytes, String> {
    let normalized = value.trim();
    if normalized.len() != 64 {
        return Err("hash must be 64 hex chars".to_string());
    }

    let mut out = [0u8; 32];
    for i in 0..32 {
        let pair = &normalized[(i * 2)..(i * 2 + 2)];
        out[i] = u8::from_str_radix(pair, 16).map_err(|_| "invalid hex hash".to_string())?;
    }
    Ok(out)
}

fn parse_decimal(value: &str) -> Result<Decimal, String> {
    value
        .parse::<Decimal>()
        .map_err(|_| format!("invalid decimal value: {value}"))
}

#[cfg(test)]
mod tests {
    use super::verify_proof_json;
    use crate::tree::{build_poseidon_merkle_sum_tree, BalanceSnapshot};
    use rust_decimal_macros::dec;
    use serde_json::json;

    fn hash_to_hex(bytes: &[u8; 32]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(64);
        for byte in bytes {
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
        }
        out
    }

    #[test]
    fn verify_proof_json_accepts_valid_merkle_path() {
        let snapshots = vec![
            BalanceSnapshot { user_id: 1, balance: dec!(10) },
            BalanceSnapshot { user_id: 2, balance: dec!(7.5) },
            BalanceSnapshot { user_id: 3, balance: dec!(3.25) },
        ];

        let tree = build_poseidon_merkle_sum_tree(&snapshots).expect("tree build must succeed");
        let proof = tree.generate_proof(1).expect("proof generation must succeed");

        let proof_json = json!({
            "user_id": snapshots[1].user_id,
            "leaf_balance": snapshots[1].balance.to_string(),
            "root_hash": hash_to_hex(&proof.root.hash),
            "root_balance": proof.root.balance.to_string(),
            "merkle_path": proof.path.iter().map(|step| json!({
                "sibling_hash": hash_to_hex(&step.sibling_hash),
                "sibling_balance": step.sibling_balance.to_string(),
                "sibling_is_left": step.sibling_is_left,
            })).collect::<Vec<_>>(),
        })
        .to_string();

        let public_inputs_json = json!({
            "expected_root_hash": hash_to_hex(&proof.root.hash),
            "expected_root_balance": proof.root.balance.to_string(),
            "expected_user_id": snapshots[1].user_id,
        })
        .to_string();

        assert!(verify_proof_json(&proof_json, &public_inputs_json));
    }

    #[test]
    fn verify_proof_json_rejects_tampered_root_hash() {
        let proof_json = json!({
            "user_id": 7,
            "leaf_balance": "10",
            "root_hash": "00".repeat(32),
            "root_balance": "10",
            "merkle_path": []
        })
        .to_string();

        let public_inputs_json = json!({
            "expected_root_hash": "11".repeat(32),
            "expected_root_balance": "10",
            "expected_user_id": 7,
        })
        .to_string();

        assert!(!verify_proof_json(&proof_json, &public_inputs_json));
    }
}

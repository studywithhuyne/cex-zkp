// ZKP crate: Merkle Sum Tree + ZK-Proof of Solvency
// Compiled to WebAssembly via wasm-pack for client-side proof verification

use wasm_bindgen::prelude::wasm_bindgen;

pub mod circuit; // ZK circuit constraints (hash/sum/overflow) - implemented in ZKP-04
pub mod poseidon; // Poseidon hashing primitives (ZKP-02)
pub mod tree;    // Merkle node and leaf initialization (ZKP-01)
pub mod verifier; // Wasm verifier for proof JSON + public inputs (ZKP-06)
#[cfg(not(target_arch = "wasm32"))]
pub mod snark;

#[wasm_bindgen]
pub fn verify_proof(proof_json: &str, public_inputs_json: &str) -> bool {
	verifier::verify_proof_json(proof_json, public_inputs_json)
}

// ZKP crate: Merkle Sum Tree + ZK-Proof of Solvency
// Compiled to WebAssembly via wasm-pack for client-side proof verification

pub mod circuit; // ZK circuit constraints (hash/sum/overflow) - implemented in ZKP-04
pub mod poseidon; // Poseidon hashing primitives (ZKP-02)
pub mod tree;    // Merkle node and leaf initialization (ZKP-01)

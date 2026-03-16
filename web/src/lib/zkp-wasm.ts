// zkp-wasm.ts — Dynamic loader for the wasm-pack compiled ZK verifier.
//
// This project keeps wasm-pack output under src/lib/zkp-wasm/.
// We import that module directly so Vite can bundle both zkp.js and zkp_bg.wasm
// for dev server and Docker/Nginx production builds.

type VerifyFn = (proofJson: string, publicInputsJson: string) => boolean;

let _verify: VerifyFn | null = null;
let _initPromise: Promise<void> | null = null;

/**
 * Load and initialize the ZKP Wasm module.
 * Safe to call multiple times — the module is cached after the first load.
 * Throws if the wasm-pack artifact is missing from src/lib/zkp-wasm/.
 */
export async function loadWasmVerifier(): Promise<void> {
  if (_verify !== null) return;
  if (_initPromise !== null) return _initPromise;

  _initPromise = (async () => {
    // Import from src so Vite rewrites/ships the companion .wasm file correctly.
    const mod: any = await import("./zkp-wasm/zkp.js");

    // wasm-pack --target web exports `init` as the default export.
    // Calling it fetches and instantiates the companion .wasm binary.
    await mod.default();

    _verify = mod.verify_proof as VerifyFn;
  })();

  return _initPromise;
}

/**
 * Run the ZK Merkle-path verifier in the browser (pure Rust logic via WASM).
 *
 * @param proofJson       JSON string matching the /api/zkp/proof response:
 *                        { user_id, leaf_balance, root_hash, root_balance, merkle_path }
 * @param publicInputsJson JSON string with the claimed public state to verify against:
 *                        { expected_root_hash, expected_root_balance, expected_user_id?: string }
 * @returns true  if the Merkle path re-computation matches the declared root.
 *          false if any hash or balance is inconsistent.
 */
export function zkpVerify(proofJson: string, publicInputsJson: string): boolean {
  if (_verify === null) {
    throw new Error("ZKP WASM not loaded. Await loadWasmVerifier() before calling zkpVerify().");
  }
  return _verify(proofJson, publicInputsJson);
}

/** Returns true once the WASM module has been successfully initialized. */
export function isWasmReady(): boolean {
  return _verify !== null;
}

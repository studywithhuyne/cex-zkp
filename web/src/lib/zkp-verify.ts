// zkp-verify.ts — Shared validation and WASM verification logic for ZKP panels.

import { loadWasmVerifier, zkpVerify } from "./zkp-wasm";
import type { ProofPayload, RawProofPayload } from "./zkp-types";

type ValidationOk = { ok: true; payload: ProofPayload };
type ValidationErr = { ok: false; error: string; isInvalid?: boolean };

/**
 * Parse and validate a raw JSON string into a normalized ProofPayload.
 */
export function validateProofJson(rawJson: string): ValidationOk | ValidationErr {
  let rawParsed: RawProofPayload;
  try {
    rawParsed = JSON.parse(rawJson);
  } catch {
    return { ok: false, error: "Invalid JSON — could not parse proof data" };
  }

  const missing: string[] = [];
  if (rawParsed.user_id === undefined || rawParsed.user_id === null || rawParsed.user_id === "")
    missing.push("user_id");
  if (typeof rawParsed.leaf_balance !== "string") missing.push("leaf_balance");
  if (typeof rawParsed.root_hash !== "string") missing.push("root_hash");
  if (typeof rawParsed.root_balance !== "string") missing.push("root_balance");
  if (!Array.isArray(rawParsed.merkle_path)) missing.push("merkle_path");

  if (missing.length > 0) {
    return {
      ok: false,
      error: `Proof JSON missing required fields: ${missing.join(", ")}`,
      isInvalid: true,
    };
  }

  const normalizedUserId = rawParsed.user_id;
  const userIdIsValid =
    (typeof normalizedUserId === "number" &&
      Number.isSafeInteger(normalizedUserId) &&
      normalizedUserId > 0) ||
    (typeof normalizedUserId === "string" &&
      /^\d+$/.test(normalizedUserId.trim()) &&
      normalizedUserId.trim() !== "0");

  if (!userIdIsValid) {
    return {
      ok: false,
      error: "Invalid user_id: expected positive u64 (safe number or numeric string)",
      isInvalid: true,
    };
  }

  const parsed: ProofPayload = {
    user_id: String(normalizedUserId).trim(),
    leaf_balance: rawParsed.leaf_balance!,
    root_hash: rawParsed.root_hash!,
    root_balance: rawParsed.root_balance!,
    merkle_path: rawParsed.merkle_path!,
    ...(rawParsed.public_inputs
      ? {
          public_inputs: {
            ...rawParsed.public_inputs,
            ...(rawParsed.public_inputs.expected_user_id !== undefined
              ? { expected_user_id: String(rawParsed.public_inputs.expected_user_id).trim() }
              : {}),
          },
        }
      : {}),
    ...(rawParsed.solvency ? { solvency: rawParsed.solvency } : {}),
  };

  return { ok: true, payload: parsed };
}

/**
 * Run WASM-based Merkle path verification on a validated ProofPayload.
 * Loads the WASM module on first call (idempotent).
 *
 * @param parsed       A validated ProofPayload
 * @param coldWalletFallback  Fallback cold_wallet_assets value when public_inputs lacks one
 * @returns { valid: true } or { valid: false, reason: string }
 * @throws on WASM loading / instantiation failure
 */
export async function runWasmVerification(
  parsed: ProofPayload,
  coldWalletFallback?: string,
): Promise<{ valid: true } | { valid: false; reason: string }> {
  await loadWasmVerifier();

  const publicInputsJson = parsed.public_inputs
    ? JSON.stringify(parsed.public_inputs)
    : JSON.stringify({
        expected_root_hash: parsed.root_hash,
        expected_root_balance: parsed.root_balance,
        expected_user_id: parsed.user_id,
        expected_cold_wallet_assets: coldWalletFallback,
      });

  const result = zkpVerify(JSON.stringify(parsed), publicInputsJson);

  if (result) {
    return { valid: true };
  }

  if (parsed.solvency && !parsed.solvency.liabilities_leq_assets) {
    return {
      valid: false,
      reason: "Solvency check failed — total liabilities exceed declared cold wallet assets",
    };
  }
  return {
    valid: false,
    reason: "Merkle path verification failed — the proof is cryptographically invalid",
  };
}

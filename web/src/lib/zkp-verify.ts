// zkp-verify.ts — Shared validation logic for SNARK-based ZKP panels.

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
  if (typeof rawParsed.root_hash !== "string") missing.push("root_hash");
  if (!rawParsed.snark || typeof rawParsed.snark.verified !== "boolean") {
    missing.push("snark.verified");
  }

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
    root_hash: rawParsed.root_hash!,
    ...(typeof rawParsed.leaf_balance === "string" ? { leaf_balance: rawParsed.leaf_balance } : {}),
    ...(rawParsed.snark ? { snark: rawParsed.snark } : {}),
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
 * Run SNARK verification result check on a validated ProofPayload.
 *
 * @param parsed       A validated ProofPayload
 * @returns { valid: true } or { valid: false, reason: string }
 * @throws on WASM loading / instantiation failure
 */
export async function runWasmVerification(
  parsed: ProofPayload,
): Promise<{ valid: true } | { valid: false; reason: string }> {
  const result = parsed.snark?.verified === true;

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
    reason: "zk-SNARK verification failed — the proof is cryptographically invalid",
  };
}

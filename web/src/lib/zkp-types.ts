// zkp-types.ts — Shared TypeScript types for ZKP verification panels.

export type ProofPayload = {
  user_id: string;
  leaf_balance: string;
  root_hash: string;
  merkle_path: unknown[];
  public_inputs?: {
    expected_root_hash: string;
    expected_user_id?: string;
  };
  solvency?: {
    liabilities_leq_assets: boolean;
    verified_at?: string;
  };
};

export type RawProofPayload = {
  user_id?: number | string;
  leaf_balance?: string;
  root_hash?: string;
  merkle_path?: unknown[];
  public_inputs?: ProofPayload["public_inputs"];
  solvency?: ProofPayload["solvency"];
};

export type VerifyStatus = "idle" | "fetching" | "verifying" | "valid" | "invalid" | "error";

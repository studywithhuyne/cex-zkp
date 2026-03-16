// zkp-types.ts — Shared TypeScript types for ZKP verification panels.

export type ProofPayload = {
  user_id: string;
  leaf_balance: string;
  root_hash: string;
  root_balance: string;
  merkle_path: unknown[];
  public_inputs?: {
    expected_root_hash: string;
    expected_root_balance: string;
    expected_user_id?: string;
    expected_cold_wallet_assets?: string;
  };
  solvency?: {
    total_liabilities: string;
    cold_wallet_assets: string;
    liabilities_leq_assets: boolean;
  };
};

export type RawProofPayload = {
  user_id?: number | string;
  leaf_balance?: string;
  root_hash?: string;
  root_balance?: string;
  merkle_path?: unknown[];
  public_inputs?: ProofPayload["public_inputs"];
  solvency?: ProofPayload["solvency"];
};

export type VerifyStatus = "idle" | "fetching" | "verifying" | "valid" | "invalid" | "error";

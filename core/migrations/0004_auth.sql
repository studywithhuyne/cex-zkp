-- Migration: 0004_auth
-- Pillar 1: Identity & Access Management
-- Adds password authentication (Argon2id) and Ed25519 API key support.

-- ──────────────────────────────────────────────────────────────────────────────
-- ALTER users: add password_hash column
-- Nullable because existing seed users were created without passwords.
-- New registrations MUST have a password_hash set.
-- ──────────────────────────────────────────────────────────────────────────────

ALTER TABLE users ADD COLUMN IF NOT EXISTS password_hash TEXT;

-- ──────────────────────────────────────────────────────────────────────────────
-- TABLE: api_keys
-- Stores Ed25519 public keys for algorithmic trader (bot) authentication.
-- Each user can have multiple API keys identified by a unique hex ID.
-- ──────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS api_keys (
    id         TEXT        PRIMARY KEY,           -- hex-encoded random 16-byte ID
    user_id    BIGINT      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    public_key BYTEA       NOT NULL,              -- 32-byte Ed25519 public key
    label      TEXT        NOT NULL DEFAULT '',    -- user-provided label (e.g. "my-bot-1")
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Fast lookup of all API keys belonging to a user.
CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys (user_id);

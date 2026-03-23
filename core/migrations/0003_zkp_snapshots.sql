-- Track ZKP Merkle Tree Snapshots
CREATE TABLE IF NOT EXISTS zkp_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    snapshot_id TEXT NOT NULL UNIQUE,
    root_hash TEXT NOT NULL,
    users_included INT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

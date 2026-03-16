-- Migration: 0002_seed_data
-- Seeds 4 mock users, 2 tradable assets (BTC / USDT), and generous initial balances.
-- Runs automatically via sqlx::migrate!() on server startup (idempotent via ON CONFLICT).
-- These user IDs (1-4) match the mock users in the Svelte frontend.

-- ──────────────────────────────────────────────────────────────────────────────
-- USERS  (x-user-id header values used by the frontend)
-- ──────────────────────────────────────────────────────────────────────────────
INSERT INTO users (id, username) VALUES
    (1, 'alice'),
    (2, 'bob'),
    (3, 'charlie'),
    (4, 'dave')
ON CONFLICT (id) DO NOTHING;

-- ──────────────────────────────────────────────────────────────────────────────
-- ASSETS
-- ──────────────────────────────────────────────────────────────────────────────
INSERT INTO assets (symbol, name, decimals) VALUES
    ('BTC',  'Bitcoin',    8),
    ('USDT', 'Tether USD', 2)
ON CONFLICT (symbol) DO NOTHING;

-- ──────────────────────────────────────────────────────────────────────────────
-- BALANCES  — 100 BTC + 10 000 000 USDT per user
-- Large enough to let any user place realistic Binance-scale orders without
-- worrying about insufficient-balance errors during demos.
-- ──────────────────────────────────────────────────────────────────────────────
INSERT INTO balances (user_id, asset_symbol, available, locked) VALUES
    -- alice (ID 1)
    (1, 'BTC',  100.00000000, 0),
    (1, 'USDT', 10000000.00000000, 0),
    -- bob (ID 2)
    (2, 'BTC',  100.00000000, 0),
    (2, 'USDT', 10000000.00000000, 0),
    -- charlie (ID 3)
    (3, 'BTC',  100.00000000, 0),
    (3, 'USDT', 10000000.00000000, 0),
    -- dave (ID 4)
    (4, 'BTC',  100.00000000, 0),
    (4, 'USDT', 10000000.00000000, 0)
ON CONFLICT (user_id, asset_symbol) DO NOTHING;

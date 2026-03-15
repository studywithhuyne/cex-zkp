-- Migration: 0001_initial_schema
-- Creates all tables for the CEX persistence layer.
-- These are write-once audit logs and balance snapshots;
-- the live matching runs 100% in-memory and never touches these tables on the hot path.

-- ──────────────────────────────────────────────────────────────────────────────
-- ENUM TYPES
-- ──────────────────────────────────────────────────────────────────────────────

CREATE TYPE order_side AS ENUM ('buy', 'sell');
CREATE TYPE order_status AS ENUM ('open', 'partial', 'filled', 'cancelled');

-- ──────────────────────────────────────────────────────────────────────────────
-- TABLE: users
-- Stores user identities keyed by the dummy x-user-id (u64 cast to BIGINT).
-- ──────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS users (
    id         BIGINT      PRIMARY KEY,           -- matches x-user-id header (u64)
    username   TEXT        NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ──────────────────────────────────────────────────────────────────────────────
-- TABLE: assets
-- Registry of tradable tokens supported by the exchange (e.g. BTC, USDT).
-- ──────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS assets (
    symbol     TEXT        PRIMARY KEY,           -- e.g. 'BTC', 'USDT'
    name       TEXT        NOT NULL,              -- e.g. 'Bitcoin'
    decimals   SMALLINT    NOT NULL DEFAULT 8,    -- display precision
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ──────────────────────────────────────────────────────────────────────────────
-- TABLE: balances
-- Current available + locked balance per (user, asset) pair.
-- Updated asynchronously; never read during live order matching.
-- ──────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS balances (
    user_id      BIGINT        NOT NULL REFERENCES users(id),
    asset_symbol TEXT          NOT NULL REFERENCES assets(symbol),
    available    NUMERIC(30,8) NOT NULL DEFAULT 0,
    locked       NUMERIC(30,8) NOT NULL DEFAULT 0,
    updated_at   TIMESTAMPTZ   NOT NULL DEFAULT now(),

    PRIMARY KEY (user_id, asset_symbol),

    CONSTRAINT chk_available_ge_zero CHECK (available >= 0),
    CONSTRAINT chk_locked_ge_zero    CHECK (locked    >= 0)
);

-- ──────────────────────────────────────────────────────────────────────────────
-- TABLE: orders_log
-- Immutable append-only audit log of every order submitted to the engine.
-- status and filled columns are updated when fills arrive via background worker.
-- ──────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS orders_log (
    id          UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id    BIGINT        NOT NULL UNIQUE,    -- matches Order.id in engine
    user_id     BIGINT        NOT NULL REFERENCES users(id),
    side        order_side    NOT NULL,
    price       NUMERIC(30,8) NOT NULL,
    amount      NUMERIC(30,8) NOT NULL,
    filled      NUMERIC(30,8) NOT NULL DEFAULT 0,
    status      order_status  NOT NULL DEFAULT 'open',
    base_asset  TEXT          NOT NULL,           -- e.g. 'BTC'
    quote_asset TEXT          NOT NULL,           -- e.g. 'USDT'
    created_at  TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ   NOT NULL DEFAULT now(),

    CONSTRAINT chk_price_positive  CHECK (price  > 0),
    CONSTRAINT chk_amount_positive CHECK (amount > 0),
    CONSTRAINT chk_filled_valid    CHECK (filled >= 0 AND filled <= amount)
);

-- ──────────────────────────────────────────────────────────────────────────────
-- TABLE: trades_log
-- Immutable record of every matched execution (fill event from the engine).
-- Written once by the async background worker; never updated afterwards.
-- ──────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS trades_log (
    id             UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
    maker_order_id BIGINT        NOT NULL REFERENCES orders_log(order_id),
    taker_order_id BIGINT        NOT NULL REFERENCES orders_log(order_id),
    maker_user_id  BIGINT        NOT NULL REFERENCES users(id),
    taker_user_id  BIGINT        NOT NULL REFERENCES users(id),
    price          NUMERIC(30,8) NOT NULL,
    amount         NUMERIC(30,8) NOT NULL,
    base_asset     TEXT          NOT NULL,
    quote_asset    TEXT          NOT NULL,
    executed_at    TIMESTAMPTZ   NOT NULL DEFAULT now(),

    CONSTRAINT chk_trade_price_positive  CHECK (price  > 0),
    CONSTRAINT chk_trade_amount_positive CHECK (amount > 0),
    CONSTRAINT chk_no_self_trade         CHECK (maker_user_id != taker_user_id)
);

-- ──────────────────────────────────────────────────────────────────────────────
-- INDEXES — optimised for the most common query patterns
-- ──────────────────────────────────────────────────────────────────────────────

-- Fetch all orders for a user (order history page)
CREATE INDEX idx_orders_log_user_id  ON orders_log (user_id);
-- Filter by order lifecycle state (open-order book view)
CREATE INDEX idx_orders_log_status   ON orders_log (status);
-- Trade history per user (both sides)
CREATE INDEX idx_trades_log_maker    ON trades_log (maker_user_id);
CREATE INDEX idx_trades_log_taker    ON trades_log (taker_user_id);
-- Recent trade feed (DESC so newest rows are found first)
CREATE INDEX idx_trades_log_exec_at  ON trades_log (executed_at DESC);

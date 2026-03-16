-- Migration: 0002_ohlcv
-- Creates the candles table for OHLCV (Open/High/Low/Close/Volume) data.
--
-- Design:
--   The matching engine never writes here directly — preserving micro-second latency.
--   The async persistence worker aggregates Trade fills into candle rows via UPSERT.
--   Supported intervals: 1m (60s), 5m (300s), 1h (3600s), 1d (86400s).
--   open_time is always floored to the start of the interval (UTC).

-- ──────────────────────────────────────────────────────────────────────────────
-- TABLE: candles
-- ──────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS candles (
    symbol    TEXT          NOT NULL,           -- e.g. 'BTC_USDT'
    interval  TEXT          NOT NULL,           -- '1m' | '5m' | '1h' | '1d'
    open_time TIMESTAMPTZ   NOT NULL,           -- start of candle period (UTC, floored)
    open      NUMERIC(30,8) NOT NULL,           -- price of the first trade in interval
    high      NUMERIC(30,8) NOT NULL,           -- highest trade price in interval
    low       NUMERIC(30,8) NOT NULL,           -- lowest trade price in interval
    close     NUMERIC(30,8) NOT NULL,           -- price of the most recent trade in interval
    volume    NUMERIC(30,8) NOT NULL,           -- total trade quantity in interval

    PRIMARY KEY (symbol, interval, open_time),

    CONSTRAINT chk_candle_open_positive   CHECK (open   > 0),
    CONSTRAINT chk_candle_high_ge_low     CHECK (high  >= low),
    CONSTRAINT chk_candle_volume_positive CHECK (volume > 0)
);

-- ──────────────────────────────────────────────────────────────────────────────
-- INDEXES
-- ──────────────────────────────────────────────────────────────────────────────

-- Primary chart query: fetch candles for one symbol/interval sorted by time
CREATE INDEX idx_candles_symbol_interval_time
    ON candles (symbol, interval, open_time DESC);

-- 00_extensions.sql
-- Runs once on first PostgreSQL container start.
-- Enables required extensions before sqlx migrations run.

-- UUID generation (used for trade_id, order_id primary keys)
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Set default timezone for this database
ALTER DATABASE cex_db SET timezone TO 'UTC';

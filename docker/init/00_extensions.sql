-- 00_extensions.sql
-- Runs once on first PostgreSQL container start.

-- Set default timezone for this database
ALTER DATABASE cex_db SET timezone TO 'UTC';

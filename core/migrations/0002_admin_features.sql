-- Add is_suspended directly to users
ALTER TABLE users ADD COLUMN is_suspended BOOLEAN NOT NULL DEFAULT FALSE;

-- Add is_active to markets and assets 
ALTER TABLE markets ADD COLUMN is_active BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE assets ADD COLUMN is_active BOOLEAN NOT NULL DEFAULT TRUE;

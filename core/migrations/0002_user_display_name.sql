ALTER TABLE users
  ADD COLUMN IF NOT EXISTS display_name TEXT;

UPDATE users
SET display_name = username
WHERE display_name IS NULL OR btrim(display_name) = '';

ALTER TABLE users
  ALTER COLUMN display_name SET NOT NULL;

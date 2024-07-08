-- Your SQL goes here
-- add `index_file` column to `sites` table
ALTER TABLE sites
    ADD COLUMN index_file VARCHAR(255);

-- rename subdomain column to host
ALTER TABLE sites
    RENAME COLUMN subdomain TO host;
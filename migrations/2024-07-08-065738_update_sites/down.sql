-- This file should undo anything in `up.sql`
ALTER TABLE sites
    DROP COLUMN index_file;
ALTER TABLE sites
    RENAME COLUMN host TO subdomain;
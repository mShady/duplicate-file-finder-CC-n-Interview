-- Add kept_path column to track which duplicate copy was retained when a file was deleted
ALTER TABLE deletion_history ADD COLUMN kept_path TEXT;

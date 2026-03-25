-- Change duplicate_groups UNIQUE constraint from (hash) to (hash, scan_session_id).
-- This prevents cross-session contamination: each scan session owns its own group rows,
-- so a re-scan cannot silently steal or overwrite another session's groups.
--
-- SQLite does not support ALTER TABLE ... DROP CONSTRAINT, so we recreate the table.

PRAGMA foreign_keys = OFF;

CREATE TABLE duplicate_groups_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    hash TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    file_count INTEGER NOT NULL DEFAULT 0,
    wasted_space INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    scan_session_id INTEGER,
    UNIQUE (hash, scan_session_id),
    FOREIGN KEY (scan_session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

INSERT INTO duplicate_groups_new (id, hash, file_size, file_count, wasted_space, created_at, scan_session_id)
    SELECT id, hash, file_size, file_count, wasted_space, created_at, scan_session_id
    FROM duplicate_groups;

DROP TABLE duplicate_groups;

ALTER TABLE duplicate_groups_new RENAME TO duplicate_groups;

-- Recreate indexes (the old UNIQUE index on hash alone is gone; the new composite UNIQUE
-- constraint automatically creates an index on (hash, scan_session_id))
CREATE INDEX IF NOT EXISTS idx_duplicate_groups_wasted_space ON duplicate_groups(wasted_space DESC);

PRAGMA foreign_keys = ON;

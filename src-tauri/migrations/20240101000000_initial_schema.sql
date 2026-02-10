-- Initial database schema for DupliFind

-- Scan sessions track individual scan operations
CREATE TABLE IF NOT EXISTS scan_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at INTEGER NOT NULL,
    completed_at INTEGER,
    status TEXT NOT NULL DEFAULT 'running',
    scanned_paths TEXT NOT NULL DEFAULT '[]',
    total_files INTEGER NOT NULL DEFAULT 0,
    total_size INTEGER NOT NULL DEFAULT 0,
    duplicate_groups INTEGER NOT NULL DEFAULT 0,
    wasted_space INTEGER NOT NULL DEFAULT 0
);

-- Duplicate groups contain files with identical content
CREATE TABLE IF NOT EXISTS duplicate_groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    hash TEXT NOT NULL UNIQUE,
    file_size INTEGER NOT NULL,
    file_count INTEGER NOT NULL DEFAULT 0,
    wasted_space INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    scan_session_id INTEGER,
    FOREIGN KEY (scan_session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

-- Scanned files with their metadata and hashes
CREATE TABLE IF NOT EXISTS scanned_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    size INTEGER NOT NULL,
    partial_hash TEXT,
    full_hash TEXT,
    created_at INTEGER NOT NULL,
    modified_at INTEGER NOT NULL,
    scanned_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    group_id INTEGER,
    scan_session_id INTEGER,
    FOREIGN KEY (group_id) REFERENCES duplicate_groups(id) ON DELETE SET NULL,
    FOREIGN KEY (scan_session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_scanned_files_size ON scanned_files(size);
CREATE INDEX IF NOT EXISTS idx_scanned_files_partial_hash ON scanned_files(partial_hash);
CREATE INDEX IF NOT EXISTS idx_scanned_files_full_hash ON scanned_files(full_hash);
CREATE INDEX IF NOT EXISTS idx_scanned_files_group_id ON scanned_files(group_id);
-- Note: No explicit index needed on duplicate_groups(hash) because the UNIQUE constraint
-- already creates an implicit index
CREATE INDEX IF NOT EXISTS idx_duplicate_groups_wasted_space ON duplicate_groups(wasted_space DESC);

-- User settings (key-value store)
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Protected folders that cannot be deleted from
CREATE TABLE IF NOT EXISTS protected_folders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    added_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Deletion history for audit and undo information
CREATE TABLE IF NOT EXISTS deletion_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    file_hash TEXT NOT NULL,
    deleted_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    group_id INTEGER,
    original_created_at INTEGER,
    original_modified_at INTEGER
);

CREATE INDEX IF NOT EXISTS idx_deletion_history_deleted_at ON deletion_history(deleted_at DESC);

-- File cache for incremental scanning
CREATE TABLE IF NOT EXISTS file_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    size INTEGER NOT NULL,
    modified_at INTEGER NOT NULL,
    partial_hash TEXT NOT NULL,
    full_hash TEXT,
    cached_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Note: No explicit index needed on file_cache(path) because the UNIQUE constraint
-- already creates an implicit index
CREATE INDEX IF NOT EXISTS idx_file_cache_size_mtime ON file_cache(size, modified_at);

-- Scan progress tracking (for pause/resume)
CREATE TABLE IF NOT EXISTS scan_progress (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    scan_session_id INTEGER NOT NULL UNIQUE,
    current_path TEXT,
    pending_paths TEXT NOT NULL DEFAULT '[]',
    processed_count INTEGER NOT NULL DEFAULT 0,
    skipped_count INTEGER NOT NULL DEFAULT 0,
    error_count INTEGER NOT NULL DEFAULT 0,
    skipped_files TEXT NOT NULL DEFAULT '[]',
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (scan_session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

-- Insert default settings
INSERT OR IGNORE INTO settings (key, value) VALUES
    ('theme', 'system'),
    ('parallelism', 'normal'),
    ('last_scan_paths', '[]'),
    ('window_width', '1200'),
    ('window_height', '800');

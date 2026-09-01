CREATE TABLE IF NOT EXISTS firmware_update_history (
    job_id TEXT PRIMARY KEY,
    batch_id TEXT,
    operation_type TEXT,
    group_name TEXT,
    receiver_ip TEXT NOT NULL,
    receiver_name TEXT,
    receiver_model TEXT NOT NULL,
    target_file TEXT NOT NULL,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    result TEXT NOT NULL,
    final_firmware_version TEXT,
    clone_file_id TEXT UNIQUE,
    errors_json TEXT NOT NULL DEFAULT '[]',
    row_version INTEGER NOT NULL DEFAULT 1,
    created_by TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_by TEXT,
    deleted_at TEXT,
    FOREIGN KEY (clone_file_id) REFERENCES clone_files(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS ix_firmware_history_started_at
    ON firmware_update_history(started_at DESC);

CREATE INDEX IF NOT EXISTS ix_firmware_history_receiver_ip
    ON firmware_update_history(receiver_ip, started_at DESC);

CREATE INDEX IF NOT EXISTS ix_firmware_history_batch_id
    ON firmware_update_history(batch_id, started_at DESC);

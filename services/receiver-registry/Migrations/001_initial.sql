CREATE TABLE IF NOT EXISTS receiver_groups (
    id INTEGER PRIMARY KEY,
    code TEXT NOT NULL COLLATE NOCASE UNIQUE,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE,
    description TEXT,
    is_system INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0, 1)),
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    row_version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS receivers (
    id TEXT PRIMARY KEY,
    ip_address TEXT NOT NULL COLLATE NOCASE UNIQUE,
    name TEXT,
    model TEXT,
    serial_number TEXT,
    location TEXT,
    notes TEXT,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    row_version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS receiver_group_memberships (
    receiver_id TEXT NOT NULL,
    group_id INTEGER NOT NULL,
    is_primary INTEGER NOT NULL DEFAULT 0 CHECK (is_primary IN (0, 1)),
    created_at TEXT NOT NULL,
    PRIMARY KEY (receiver_id, group_id),
    FOREIGN KEY (receiver_id) REFERENCES receivers(id) ON DELETE CASCADE,
    FOREIGN KEY (group_id) REFERENCES receiver_groups(id) ON DELETE RESTRICT
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_receiver_primary_group
    ON receiver_group_memberships(receiver_id)
    WHERE is_primary = 1;

CREATE INDEX IF NOT EXISTS ix_memberships_group_id
    ON receiver_group_memberships(group_id);

CREATE INDEX IF NOT EXISTS ix_receivers_model
    ON receivers(model);

CREATE TABLE IF NOT EXISTS clone_files (
    id TEXT PRIMARY KEY,
    receiver_id TEXT,
    file_name TEXT NOT NULL,
    relative_path TEXT NOT NULL UNIQUE,
    size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
    sha256 TEXT NOT NULL,
    firmware_version TEXT,
    description TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (receiver_id) REFERENCES receivers(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS ix_clone_files_receiver_id
    ON clone_files(receiver_id);

CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    action TEXT NOT NULL,
    actor TEXT NOT NULL,
    changed_at TEXT NOT NULL,
    payload_json TEXT
);

CREATE INDEX IF NOT EXISTS ix_audit_log_changed_at
    ON audit_log(changed_at DESC);

CREATE INDEX IF NOT EXISTS ix_audit_log_entity
    ON audit_log(entity_type, entity_id, changed_at DESC);

INSERT OR IGNORE INTO receiver_groups
    (id, code, name, description, is_system, enabled, row_version, created_at, updated_at)
VALUES
    (1, 'BETA', 'Beta', 'Beta receiver group', 1, 1, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    (2, 'ALPHA', 'Alpha', 'Alpha receiver group', 1, 1, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    (3, 'GROUP_3', 'Group 3', NULL, 1, 1, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    (4, 'GROUP_4', 'Group 4', NULL, 1, 1, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

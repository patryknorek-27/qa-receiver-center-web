CREATE TABLE IF NOT EXISTS app_settings (
    setting_key TEXT PRIMARY KEY COLLATE NOCASE,
    value_json TEXT NOT NULL CHECK (json_valid(value_json)),
    row_version INTEGER NOT NULL DEFAULT 1,
    updated_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

INSERT OR IGNORE INTO app_settings
    (setting_key, value_json, row_version, updated_by, created_at, updated_at)
VALUES
    (
        'update-safety',
        '{"postRestartDelayMinutes":2,"preRestartSafetyMarginMinutes":10,"restartCycleMinutes":30}',
        1,
        'database-migration',
        CURRENT_TIMESTAMP,
        CURRENT_TIMESTAMP
    );

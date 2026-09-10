INSERT OR IGNORE INTO app_settings
    (setting_key, value_json, row_version, updated_by, created_at, updated_at)
VALUES
    (
        'ip-monitoring',
        '{"automaticScanEnabled":true,"intervalMinutes":10,"timeoutMilliseconds":1000}',
        1,
        'database-migration',
        CURRENT_TIMESTAMP,
        CURRENT_TIMESTAMP
    );

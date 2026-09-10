ALTER TABLE ip_inventory ADD COLUMN usage_end_date TEXT;

ALTER TABLE ip_requests ADD COLUMN usage_end_date TEXT;

CREATE INDEX IF NOT EXISTS ix_ip_inventory_usage_end_date
    ON ip_inventory(usage_end_date)
    WHERE usage_end_date IS NOT NULL;

INSERT INTO audit_log
    (entity_type, entity_id, action, actor, changed_at, payload_json)
VALUES
    ('database-schema', 'ip-usage-period', 'created', 'database-migration', CURRENT_TIMESTAMP,
     '{"version":7,"description":"Added an optional usage end date to IP addresses and requests."}');

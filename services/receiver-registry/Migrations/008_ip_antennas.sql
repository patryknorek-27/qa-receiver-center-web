ALTER TABLE ip_inventory ADD COLUMN antenna_1 TEXT;

ALTER TABLE ip_inventory ADD COLUMN antenna_2 TEXT;

ALTER TABLE ip_requests ADD COLUMN antenna_1 TEXT;

ALTER TABLE ip_requests ADD COLUMN antenna_2 TEXT;

INSERT INTO audit_log
    (entity_type, entity_id, action, actor, changed_at, payload_json)
VALUES
    ('database-schema', 'ip-antennas', 'created', 'database-migration', CURRENT_TIMESTAMP,
     '{"version":8,"description":"Added two optional antenna fields to IP addresses and requests."}');

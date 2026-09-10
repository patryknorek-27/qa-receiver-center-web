CREATE TABLE IF NOT EXISTS ip_inventory (
    ip_address TEXT PRIMARY KEY COLLATE NOCASE,
    sort_order INTEGER NOT NULL UNIQUE CHECK (sort_order BETWEEN 1 AND 254),
    status TEXT NOT NULL DEFAULT 'Unused' CHECK (status IN ('Unused', 'Used', 'Reserved')),
    location TEXT,
    responsible TEXT,
    environment TEXT CHECK (
        environment IS NULL OR environment IN ('Production', 'Development', 'Testing/Lab', 'Staging')
    ),
    device_type TEXT,
    serial_number TEXT,
    notes TEXT,
    is_online INTEGER CHECK (is_online IS NULL OR is_online IN (0, 1)),
    last_ping_at TEXT,
    row_version INTEGER NOT NULL DEFAULT 1 CHECK (row_version > 0),
    created_by TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (sort_order <> 1 OR (ip_address = '10.2.155.1' AND status = 'Reserved'))
);

CREATE INDEX IF NOT EXISTS ix_ip_inventory_status
    ON ip_inventory(status, sort_order);

CREATE INDEX IF NOT EXISTS ix_ip_inventory_location
    ON ip_inventory(location COLLATE NOCASE);

CREATE INDEX IF NOT EXISTS ix_ip_inventory_responsible
    ON ip_inventory(responsible COLLATE NOCASE);

CREATE TABLE IF NOT EXISTS ip_requests (
    id TEXT PRIMARY KEY,
    ip_address TEXT NOT NULL COLLATE NOCASE,
    requested_by TEXT NOT NULL,
    location TEXT,
    responsible TEXT,
    environment TEXT CHECK (
        environment IS NULL OR environment IN ('Production', 'Development', 'Testing/Lab', 'Staging')
    ),
    device_type TEXT,
    serial_number TEXT,
    notes TEXT,
    status TEXT NOT NULL DEFAULT 'Pending' CHECK (status IN ('Pending', 'Approved', 'Rejected', 'Cancelled')),
    decision_by TEXT,
    decision_reason TEXT,
    decided_at TEXT,
    row_version INTEGER NOT NULL DEFAULT 1 CHECK (row_version > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (ip_address) REFERENCES ip_inventory(ip_address) ON DELETE RESTRICT,
    CHECK (
        (status = 'Pending' AND decision_by IS NULL AND decided_at IS NULL) OR
        (status <> 'Pending' AND decision_by IS NOT NULL AND decided_at IS NOT NULL)
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_ip_requests_one_pending_per_ip
    ON ip_requests(ip_address)
    WHERE status = 'Pending';

CREATE INDEX IF NOT EXISTS ix_ip_requests_status_created
    ON ip_requests(status, created_at DESC);

CREATE TABLE IF NOT EXISTS ip_request_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    request_id TEXT NOT NULL,
    action TEXT NOT NULL CHECK (action IN ('Created', 'Approved', 'Rejected', 'Cancelled')),
    actor TEXT NOT NULL,
    details_json TEXT CHECK (details_json IS NULL OR json_valid(details_json)),
    created_at TEXT NOT NULL,
    FOREIGN KEY (request_id) REFERENCES ip_requests(id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS ix_ip_request_events_request
    ON ip_request_events(request_id, id);

WITH RECURSIVE subnet_octets(octet) AS (
    SELECT 1
    UNION ALL
    SELECT octet + 1 FROM subnet_octets WHERE octet < 254
)
INSERT OR IGNORE INTO ip_inventory
    (ip_address, sort_order, status, location, responsible, environment, device_type,
     serial_number, notes, is_online, last_ping_at, row_version, created_by, updated_by,
     created_at, updated_at)
SELECT
    '10.2.155.' || octet,
    octet,
    CASE WHEN octet = 1 THEN 'Reserved' ELSE 'Unused' END,
    CASE WHEN octet = 1 THEN 'Network' ELSE NULL END,
    CASE WHEN octet = 1 THEN 'Network Administration' ELSE NULL END,
    NULL,
    CASE WHEN octet = 1 THEN 'Gateway' ELSE NULL END,
    NULL,
    CASE WHEN octet = 1 THEN 'Default gateway. This address cannot be requested.' ELSE NULL END,
    NULL,
    NULL,
    1,
    'database-migration',
    'database-migration',
    CURRENT_TIMESTAMP,
    CURRENT_TIMESTAMP
FROM subnet_octets;

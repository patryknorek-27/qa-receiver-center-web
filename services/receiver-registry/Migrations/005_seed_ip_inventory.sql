CREATE TEMP TABLE ip_inventory_seed_guard (
    pristine_record_count INTEGER NOT NULL CHECK (pristine_record_count = 253)
);

INSERT INTO ip_inventory_seed_guard (pristine_record_count)
SELECT COUNT(*)
FROM ip_inventory
WHERE sort_order BETWEEN 2 AND 254
  AND row_version = 1
  AND updated_by = 'database-migration';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA',
    device_type = 'PwrPi',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.2';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'Admin (Andreas)',
    device_type = 'Network Switch Netgear Prosafe',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.3';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.4';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.5';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'FW team',
    device_type = 'Alloy',
    serial_number = 'SN: 6117R40085',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.6';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'FW team',
    device_type = 'BD1090',
    serial_number = 'SN: 6552718813',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.7';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'FW team',
    device_type = 'BD1092i',
    serial_number = 'SN: STELLA2C16',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.8';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'R&D Aurel Prestel',
    device_type = 'AlloyII',
    serial_number = 'S/N: SILKC37403',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.9';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'Moxabox (4-port) Roof',
    serial_number = 'MAC: 00:90:E8:A7:02:18',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.10';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'Stella - BX1092i STELLA2C44',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.11';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'Stella  - BX1092i STELLA2C54',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.12';

UPDATE ip_inventory
SET status = 'Used',
    location = 'Attic',
    responsible = 'QA',
    device_type = 'Helium',
    serial_number = 'S/N:6038C00427',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.13';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'Alloy II - SILKC37395',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.14';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'Alloy II - SILKC37395',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.15';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'Alloy II',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.16';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.17';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.18';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.19';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'AUTO-FW',
    device_type = 'B6600T_1',
    serial_number = 'VAN IP: 10.10.10.20',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.20';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.21';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'RTK',
    device_type = 'RX750 / SPS866 / Barracuda',
    serial_number = 'S/N: 1EC09C7888, Evaluation Board',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.22';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.23';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.24';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'NetR9',
    serial_number = 'S/N: 5026K68577',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.25';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'NetR9',
    serial_number = 'S/N: 5528R50142',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.26';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.27';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.28';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'Alloy',
    serial_number = '5841R40054 (Alloy Support)',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.29';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'Spirent GSS6450 #1',
    serial_number = 'Please use with care! Deivce must be manually turned on!',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.30';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.31';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.32';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA',
    device_type = 'Raspberry Pi Roof-Metal-Box-2',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.33';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.34';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA',
    device_type = 'Raspberry Pi QA-Lab-AG',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.35';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'Markus Glocker',
    device_type = 'BD940-INS',
    serial_number = 'S/N: 5717C04072',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.36';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'RX750 / Barracuda',
    serial_number = 'S/N: 1EC09C93E5,  Evaluation Board',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.37';

UPDATE ip_inventory
SET status = 'Used',
    location = 'Attic',
    responsible = 'QA',
    device_type = 'Moxa BOX in Attic for Weather Station',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.38';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'Alloy WukongTest',
    serial_number = 'S/N 6141R40026',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.39';

UPDATE ip_inventory
SET status = 'Used',
    location = 'Attic',
    responsible = 'QA',
    device_type = 'Alloy',
    serial_number = 'S/N: 5921R40218',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.40';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'MonSol-T4D_Ramiz',
    device_type = 'RasperryPI for TS on Roof',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.41';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.42';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.43';

UPDATE ip_inventory
SET status = 'Used',
    location = 'Office roof',
    responsible = 'QA',
    device_type = 'Timo on the roof of the office',
    serial_number = 'S/N: 6616720979',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.44';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD940-INS',
    serial_number = 'S/N: 5809C00385',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.45';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD940-INS',
    serial_number = 'S/N: 5809C00356',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.46';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.47';

UPDATE ip_inventory
SET status = 'Used',
    location = 'Office roof',
    responsible = 'QA',
    device_type = 'Timo on the roof of the office',
    serial_number = 'S/N: 6624777832',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.48';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.49';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.50';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA, VAN IP : 10.10.10.51',
    device_type = 'BD992-INS',
    serial_number = 'S/N:5728C00029',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.51';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.52';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA',
    device_type = 'Novatel PwrPak7',
    serial_number = 'S/N:NMND17440002L',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.53';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD940-INS',
    serial_number = 'S/N: 5749C00120',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.54';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD940-INS',
    serial_number = 'S/N: 5749C00107',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.55';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'AUTO-FW',
    device_type = 'B5200_1',
    serial_number = 'S/N:2425000052',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.56';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD940-INS',
    serial_number = 'S/N: 5749C00031',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.57';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD940-INS',
    serial_number = 'S/N: 5749C00004',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.58';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD940-INS',
    serial_number = 'S/N: 5809C00383',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.59';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.60';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.61';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.62';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.63';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.64';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.65';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.66';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.67';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.68';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA, VAN IP : 10.10.10.69',
    device_type = 'BX992-INS',
    serial_number = 'S/N:0000000000',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.69';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BX992-INS',
    serial_number = 'S/N:5937C01364',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.70';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA, VAN IP : 10.10.10.71',
    device_type = 'BD940-INS',
    serial_number = 'S/N:5717C04020',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.71';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD940-INS',
    serial_number = 'S/N:5717C03861',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.72';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD990',
    serial_number = 'S/N:5724C01816',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.73';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA',
    device_type = 'BD990',
    serial_number = 'S/N:5724C01827',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.74';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.75';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.76';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA',
    device_type = 'DET-QA-CARPC-04',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.77';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.78';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.79';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.80';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.81';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.82';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD940-INS',
    serial_number = 'S/N:5637C00016',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.83';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA, VAN IP : 10.10.10.84',
    device_type = 'BD940-INS',
    serial_number = 'S/N:5649C00328',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.84';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.85';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.86';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.87';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.88';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'NetOps',
    device_type = 'Alloy',
    serial_number = 'S/N: 5750R40008',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.89';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA',
    device_type = 'BD940-INS',
    serial_number = 'S/N: 5849C00921',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.90';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD940',
    serial_number = 'S/N: 5837C00335',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.91';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD940',
    serial_number = 'S/N: 5837C00343',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.92';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA',
    device_type = 'MoxaBox',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.93';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.94';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.95';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.96';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.97';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.98';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.99';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'TAP FAI',
    device_type = 'FixPosition',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.100';

UPDATE ip_inventory
SET status = 'Used',
    location = 'Attic',
    responsible = 'QA',
    device_type = 'Former 10.2.155.123',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.101';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA',
    device_type = 'MoxaBox (16 COM Ports)',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.102';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA (Tilo)',
    device_type = 'Alloy',
    serial_number = 'S/N 5813R40027',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.103';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.104';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = NULL,
    device_type = 'Alloy',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.105';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.106';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD940-INS',
    serial_number = 'S/N: 5749C00132',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.107';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.108';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'AUTO-FW',
    device_type = 'FW Lab switch',
    serial_number = 'FW Lab switch',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.109';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.110';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BX-992',
    serial_number = 'S/N: 6451C41897',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.111';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BX-992',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.112';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD940-INS',
    serial_number = 'S/N: 5849C00922',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.113';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA',
    device_type = 'Swift Duro',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.114';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD940-INS',
    serial_number = 'S/N: 5809C00419',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.115';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD940-INS',
    serial_number = 'S/N: 5851C01514',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.116';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.117';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'Alloy WukongTest',
    serial_number = 'S/N 6141R40159',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.118';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'Alloy',
    serial_number = 'S/N: 5949R44112',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.119';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.120';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'GPA',
    device_type = 'BD940',
    serial_number = 'S/N: 5905C00276  at Ralf Stolz',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.121';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD940-INS',
    serial_number = 'S/N: 5905C00184',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.122';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA',
    device_type = 'Alloy',
    serial_number = 'S/N: 6148R40058',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.123';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD935-INS NOT TEMP CAL',
    serial_number = 'S/N: 5531C00062 BOXTER. SPIRENT PLAYBACKS',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.124';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.125';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA',
    device_type = 'Raspberry Pi QA-Lab-AG_2',
    serial_number = 'Raspberry',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.126';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.127';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'R750-2',
    serial_number = 'S/N: 6451R91020',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.128';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.129';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.130';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.131';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.132';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.133';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'ProPak6',
    device_type = 'Novatel',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.134';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD940-INS',
    serial_number = 'S/N:5809C00271',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.135';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'Septentrio AsteRX3',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.136';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'Infra',
    device_type = 'Nautel Transmitter',
    serial_number = 'Moxa (port 4007)',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.137';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'Infra',
    device_type = 'Raven Demodulator',
    serial_number = 'Side A; Port 2101',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.138';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'Infra',
    device_type = 'Raven Demodulator',
    serial_number = 'Side B; Port 2101',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.139';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'Infra',
    device_type = 'MSK Modulator',
    serial_number = 'NetM3 (S/N 15018); Side A',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.140';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'Infra',
    device_type = 'MSK Modulator',
    serial_number = 'NetM3 (S/N 15021); Side B',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.141';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'Infra',
    device_type = 'MSK Modulator',
    serial_number = 'NetM3 (S/N 15022)',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.142';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'Infra',
    device_type = 'MSK Modulator',
    serial_number = 'NetM3 (S/N 15019)',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.143';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA - Chronos',
    device_type = 'NetR9',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.144';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA - Chronos',
    device_type = 'NetR9',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.145';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'R750-2',
    serial_number = '6442R91013',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.146';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.147';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.148';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'Alloy WukongTest',
    serial_number = 'S/N 6141R40154',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.149';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'R750',
    serial_number = 'S/N 6304R05121',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.150';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.151';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.152';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.153';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'Infra',
    device_type = 'NetD3 Demodulator',
    serial_number = 'Beacon receiver, Near Field (S/N 5517R35008)',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.154';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.155';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD9250 Helium RevC',
    serial_number = 'S/N: 0000000000',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.156';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD940-INS',
    serial_number = 'S/N:  5905C00091. SPIRENT PLAYBACKS',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.157';

UPDATE ip_inventory
SET status = 'Used',
    location = 'AutoLab',
    responsible = 'QA',
    device_type = 'uBlox',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.158';

UPDATE ip_inventory
SET status = 'Used',
    location = 'AutoLab',
    responsible = 'QA',
    device_type = 'ublox',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.159';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'Leica GR30',
    serial_number = 'DO NOT USE FOR MOXA BOXES',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.160';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA',
    device_type = 'MoxaBox (16 COM Ports) Regal',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.161';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.162';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.163';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'NetRS',
    serial_number = 'antenna power 0309',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.164';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'NetRS',
    serial_number = 'det-fireball antenna power 0304',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.165';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'NetRS',
    serial_number = 'det-fireball-2 antenna power 0310',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.166';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.167';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.168';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA VAN IP : 10.10.10.169',
    device_type = 'BX940',
    serial_number = 'S/N: 5717C04004',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.169';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.170';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD930',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.171';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD940_INS',
    serial_number = 'S/N 5905C00230, SPIRENT PLAYBACKS',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.172';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA',
    device_type = 'BD992-INS',
    serial_number = '6306C68747',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.173';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'SPS855 (Fugro)',
    serial_number = '156.255',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.174';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'Alloy',
    serial_number = '5950R40258',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.175';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = NULL,
    device_type = 'BD992-INS',
    serial_number = '6306C68827',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.176';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'Alloy',
    serial_number = '5951R40168 -> Jammer Test',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.177';

UPDATE ip_inventory
SET status = 'Used',
    location = 'AutoLab',
    responsible = 'QA',
    device_type = 'UM980',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.178';

UPDATE ip_inventory
SET status = 'Used',
    location = 'AutoLab',
    responsible = 'QA',
    device_type = 'UM980',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.179';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD940-INS',
    serial_number = 'S/N:5849C00962',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.180';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.181';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.182';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.183';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'NetR9',
    serial_number = '157.254',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.184';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.185';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.186';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.187';

UPDATE ip_inventory
SET status = 'Used',
    location = 'AutoLab',
    responsible = 'QA',
    device_type = 'Allystar TAU951M',
    serial_number = 'Chronos Testing',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.188';

UPDATE ip_inventory
SET status = 'Used',
    location = 'AutoLab',
    responsible = 'QA',
    device_type = 'Allystar TAU951M',
    serial_number = 'Chronos Testing',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.189';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'Alloy',
    serial_number = 'S/N6412R40074',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.190';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.191';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'Alloy - Tungsten',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.192';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.193';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD992-INS',
    serial_number = 'S/N: 5833C01794',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.194';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'R750',
    serial_number = 'S/N: 6502R47518',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.195';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD992-INS',
    serial_number = '6306C68751',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.196';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'R750',
    serial_number = 'S/N: 6502R47534',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.197';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'AsteRx-U',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.198';

UPDATE ip_inventory
SET status = 'Used',
    location = 'Attic',
    responsible = 'QA',
    device_type = 'NetX1',
    serial_number = 'S/N: 04A34B69C1; First Alloy at Terrasat',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.199';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'Alloy',
    serial_number = 'Höhenkirchen 304',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.200';

UPDATE ip_inventory
SET status = 'Used',
    location = 'Attic',
    responsible = 'QA',
    device_type = 'Alloy',
    serial_number = '5951R40154',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.201';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA',
    device_type = 'Barracuda (MPS566 )',
    serial_number = '6114R06379',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.202';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.203';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.204';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.205';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD992-INS',
    serial_number = '6306C68826',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.206';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'NetR9',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.207';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD982',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.208';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'AUTO-FW',
    device_type = 'B5000 Rev. C with BisonV MM [L2]',
    serial_number = 'B5K_B5MM_L1L2_3 (Was B5K_B5MM_L1L2_1)',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.209';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.210';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.211';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'SPS',
    serial_number = '----',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.212';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.213';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'AUTO-FW',
    device_type = 'B5000 with BisonV MM [L5]',
    serial_number = 'B5K_B5MM_L1L5_2',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.214';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.215';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BD910',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.216';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'NetR5',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.217';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA',
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.218';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA',
    device_type = 'STELLA',
    serial_number = '2529.0',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.219';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.220';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.221';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.222';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA',
    device_type = 'RaspBerry Pi-Roof-Metal-Box-1',
    serial_number = 'Old',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.223';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA',
    device_type = 'TPS NetG3A',
    serial_number = 'Hoehenkirchen Topcon NetG3A',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.224';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.225';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA',
    device_type = 'RX750 Barracuda RevC -- SDR',
    serial_number = 'S/N:SFTUHFDEV1',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.226';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'Support',
    device_type = 'NetR8',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.227';

UPDATE ip_inventory
SET status = 'Used',
    location = 'AutoLab',
    responsible = 'QA',
    device_type = 'Unicore UM980',
    serial_number = 'Chronos Testing J.Mayer',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.228';

UPDATE ip_inventory
SET status = 'Used',
    location = 'AutoLab',
    responsible = 'QA',
    device_type = 'Unicore UM980',
    serial_number = 'Chronos Testing J.Mayer',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.229';

UPDATE ip_inventory
SET status = 'Unused',
    location = 'AutoLab',
    responsible = 'QA',
    device_type = 'Unicore UM980',
    serial_number = 'Chronos Testing J.Mayer',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.230';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'ublox ZED-F9P (QALAB)',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.231';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.232';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'ublox ZED-X20P (QALAB)',
    serial_number = 'S/N 04A34B84BB',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.233';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.234';

UPDATE ip_inventory
SET status = 'Used',
    location = 'Attic',
    responsible = 'QA',
    device_type = 'Helium',
    serial_number = 'SN:6038C00416',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.235';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.236';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.237';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'Helium',
    serial_number = 'SN:0000000000',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.238';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BX992',
    serial_number = 'SN: 6451C41900',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.239';

UPDATE ip_inventory
SET status = 'Unused',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.240';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.241';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'BX992',
    serial_number = 'SN: 6451C41895',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.242';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'AUTO-FW',
    device_type = 'B5000 with BisonV DR [L5]',
    serial_number = 'B5600T-5',
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.243';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = NULL,
    device_type = NULL,
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.244';

UPDATE ip_inventory
SET status = 'Used',
    location = 'Attic',
    responsible = 'QA',
    device_type = 'NetR8',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.245';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'GR10',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.246';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'RTK',
    device_type = 'Lithium BD 920',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.247';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'RTK',
    device_type = 'Morphine BD 910',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.248';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'MoxaBox (8 Com Ports) Door',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.249';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'NetR9',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.250';

UPDATE ip_inventory
SET status = 'Used',
    location = 'Attic',
    responsible = 'QA',
    device_type = 'NetR9',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.251';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'NetR9',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.252';

UPDATE ip_inventory
SET status = 'Used',
    location = 'QALab',
    responsible = 'QA',
    device_type = 'NetR9',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.253';

UPDATE ip_inventory
SET status = 'Used',
    location = NULL,
    responsible = 'QA',
    device_type = 'POE Switch!',
    serial_number = NULL,
    row_version = row_version + 1,
    updated_by = 'static-ip-overview-migration',
    updated_at = CURRENT_TIMESTAMP
WHERE ip_address = '10.2.155.254';

INSERT INTO audit_log
    (entity_type, entity_id, action, actor, changed_at, payload_json)
VALUES
    ('ip-inventory', '10.2.155.0/24', 'seeded', 'database-migration', CURRENT_TIMESTAMP,
     '{"source":"Static IP Overview.xlsx","records":253,"used":168,"unused":85}');

DROP TABLE ip_inventory_seed_guard;

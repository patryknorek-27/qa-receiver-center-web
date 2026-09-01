# QA Receiver Registry

QA Receiver Registry is the internal data service for QA Receiver Center. It keeps the shared receiver inventory in a local SQLite database on the designated server and exposes it through an API restricted to the Trimble network.

## Server layout

| Purpose | Path |
| --- | --- |
| Service binaries | `C:\TSQATools\QAReceiverRegistry\app` |
| Service configuration | `C:\TSQATools\QAReceiverRegistry\config` |
| SQLite database | `C:\TSQATools\QAReceiverRegistry\data\receivers.db` |
| Clone files | `C:\TSQATools\QAReceiverRegistry\clone-files` |
| Backups | `C:\TSQATools\Backups\QAReceiverRegistry` |

The service runs as `NT AUTHORITY\LOCAL SERVICE`. It does not connect to, modify, or depend on Microsoft SQL Server.

## Build

Requirements on the development computer:

- .NET 10 SDK
- access to `https://api.nuget.org`

From an ordinary PowerShell window in the repository:

```powershell
.\services\receiver-registry\scripts\Publish-Service.ps1
```

The self-contained Windows package is created in `services\receiver-registry\publish\win-x64`, together with a ZIP archive for transfer to the server. The target server does not need the .NET runtime or SDK.

Run the isolated integration test before publishing:

```powershell
.\services\receiver-registry\scripts\Run-IntegrationTests.ps1
```

## First server installation

Extract the deployment ZIP on `HHKNQA1-DE-DE04`. Run PowerShell as Administrator from the extracted folder, then run:

```powershell
.\Install-Service.ps1
```

The default installation uses HTTP on the dedicated server address and TCP 8787. The script validates the port, copies the service, creates the configuration, applies database migrations, registers automatic recovery, creates a Domain-profile firewall rule, starts the service, and verifies `/health`.

HTTPS remains available when a suitable server certificate is issued:

```powershell
.\Install-Service.ps1 -UseHttps -CertificateThumbprint "CERTIFICATE_THUMBPRINT"
```

No database, backup, clone file, or certificate is deleted by `Uninstall-Service.ps1`.

## Service update

For an existing installation, extract the new package and run an elevated PowerShell window from the extracted folder:

```powershell
.\Update-Service.ps1
```

The update script verifies package checksums, creates a database backup, preserves the current configuration, stores rollback binaries, stops the service, applies migrations, restarts the service, and verifies `/health`. It does not reinstall the service or replace the SQLite database.

## Verification

From a workstation inside the Trimble network:

```powershell
.\Test-Service.ps1
```

Expected values are `Status: healthy` and `Integrity: ok`.

## Initial receiver import

Run the importer from a domain workstation that can access both Azure DevOps and the registry. The first run is read-only and displays the planned changes:

```powershell
.\Import-Receivers.ps1
```

Apply the reviewed plan with:

```powershell
.\Import-Receivers.ps1 -Apply
```

The importer validates service health, reads the current `qa-receivers.json` with Windows authentication, creates a verified database backup, and then creates or updates receiver metadata records. Operational group assignments remain in `qa-receivers.json` and are shown only for review; they are not written to SQLite. Receiver usernames and passwords are neither stored in SQLite nor sent to the registry API.

## API

Base URL: `http://HHKNQA1-DE-DE04.eu.trimblecorp.net:8787`

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/health` | Service and database status |
| `GET` | `/api/v1/receivers` | Active receivers |
| `GET` | `/api/v1/receivers?includeDisabled=true` | All receivers |
| `GET` | `/api/v1/receivers/{id}` | One receiver |
| `POST` | `/api/v1/receivers` | Add a receiver |
| `PUT` | `/api/v1/receivers/{id}` | Update a receiver |
| `DELETE` | `/api/v1/receivers/{id}?rowVersion={version}` | Disable a receiver |
| `GET` | `/api/v1/groups` | Active groups |
| `POST` | `/api/v1/groups` | Add a group |
| `PUT` | `/api/v1/groups/{id}` | Update a group |
| `GET` | `/api/v1/settings` | Shared application settings |
| `GET` | `/api/v1/settings/{key}` | One shared setting |
| `PUT` | `/api/v1/settings/{key}` | Update one shared setting |
| `GET` | `/api/v1/firmware-history` | Shared firmware update history |
| `PUT` | `/api/v1/firmware-history/{jobId}` | Create or synchronize one history entry |
| `PUT` | `/api/v1/firmware-history/{jobId}/clone-file?fileName={name}` | Upload a Clone File |
| `GET` | `/api/v1/firmware-history/{jobId}/clone-file` | Download a verified Clone File |
| `DELETE` | `/api/v1/firmware-history/{jobId}` | Delete history and its Clone File |
| `GET` | `/api/v1/audit?limit=100` | Recent changes |
| `POST` | `/api/v1/maintenance/backup` | Create a verified backup |

Write requests can include `X-QA-User` with the current user's corporate identity. Every write is recorded in the audit log. Updates require the latest `rowVersion`; stale updates return HTTP `409 Conflict`.

## Operations

- SQLite runs locally in WAL mode with foreign-key enforcement.
- Shared settings use optimistic concurrency through `rowVersion` and every change is audited.
- Firmware history synchronization is idempotent, and Clone Files are limited to 20 MB and verified with SHA-256.
- Deleting a firmware history entry also removes its central Clone File and records the action in the audit log.
- A verified backup is created at service startup and every 24 hours.
- The newest 14 backups are retained by default.
- Service events are written to **Event Viewer > Windows Logs > Application** under source `QA Receiver Registry`.
- The service is restricted to the server address, TCP 8787, and the Windows Domain firewall profile.
- HTTP is used only on the internal Trimble network. Do not expose the service to the public internet or store receiver passwords in the registry while transport encryption is disabled.

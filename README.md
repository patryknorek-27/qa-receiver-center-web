# QA Receiver Center

QA Receiver Center is an internal Windows application for working with Trimble QA receivers.

The application shows receiver status, helps with firmware updates, creates configuration backups, and keeps update history in one place.

## Application Structure

![QA Receiver Center application structure](./docs/qa-receiver-center-architecture.png =700x)

The diagram shows the main parts of the current application.

1. The "User" works with "QA Receiver Center".
2. The "React UI" shows information and starts actions selected by the user.
3. "Tauri and Rust" communicates with receivers, internal systems, and the central service.
4. The "Local Cache" keeps the latest valid data when the central service is temporarily unavailable.
5. "Internal Trimble Systems" provides the receiver list, firmware packages, and access to QA receivers.
6. "QA Receiver Registry" stores shared data in SQLite and keeps Clone Files on the central computer.
7. The application connects to "QA Receiver Registry" through TCP port "8787".

## Current Versions

QA Receiver Center version: "0.3.0"

QA Receiver Registry service version: "0.7.0"

Database schema version: "8"

## Main Features

1. Loads the active receiver list from Azure DevOps using the current Windows account.
2. Shows receiver status, firmware, uptime, fix type, temperature, satellites, and errors.
3. Supports firmware updates for one receiver, many receivers, Alpha receivers, and Beta receivers.
4. Finds firmware packages in the internal release catalog.
5. Supports local ".img" and ".timg" firmware files.
6. Creates a configuration backup before a firmware update.
7. Restores receiver passwords after an update.
8. Saves firmware update history and Clone Files on the central computer.
9. Shares Update Safety settings between users.
10. Keeps a local copy of the latest valid data in case the central service is temporarily unavailable.
11. Manages static IP assignments.

## How The Data Is Stored

The active receiver list, passwords, and receiver groups come from "qa-receivers.json" in Azure DevOps. This file remains the main source for this information because receiver groups can change.

The QA Receiver Registry stores shared settings, basic receiver information, static IP assignments, change history, firmware update history, and Clone Files.

The editable firmware configuration is stored in:

"%APPDATA%\QA Receiver Center\firmware-configs.json"

Receiver passwords are not stored in SQLite and are not sent to the registry service.

## QA Receiver Registry

The registry runs as the Windows service "QAReceiverRegistry" on the internal computer prepared for the application. The service starts automatically when the computer starts.

The application connects to the service through the internal network on TCP port "8787". User computers do not open the SQLite database file directly.

The database, Clone Files, backups, and service files are stored under:

"C:\TSQATools"

The registry works separately from Microsoft SQL Server. It does not change SQL Server and does not need SQL Server to work.

Each shared setting has a version number. This protects a newer change from being replaced by an older one. Clone Files are checked with SHA256 before they are saved.

More information about installation, updates, backups, tests, and the internal API is available in "services\receiver-registry\README.md".

## What You Need

1. "Node.js 22 LTS"
2. Rust installed with rustup
3. "Build Tools for Visual Studio" with "Desktop development with C++"
4. Microsoft Edge WebView2 Runtime
5. Access to the internal Trimble network
6. Access to Azure DevOps, the firmware release server, and the QA Receiver Registry service

## Start The Application From The Repository

1. Install Node.js 22 LTS from <https://nodejs.org/en/download>.
2. Install Rust from <https://rustup.rs>.
3. Install Microsoft C++ Build Tools if Rust asks for them.
4. Double click "start-tauri-dev.bat" in the main repository folder.

The first start installs the required JavaScript packages. The first Rust build can take several minutes. Keep the command window open until the application appears.

## Troubleshooting: Node.js Not Found

If you installed "Node.js" while the terminal was already open, the terminal may not see the new system paths. Close the terminal, open it again, and start the application one more time.

## Check The Application

Run these commands from the main repository folder:

```powershell
npm run lint
npm run build
cd src-tauri
cargo test
cargo check
```

Run this command before creating a new registry service package:

```powershell
.\services\receiver-registry\scripts\Run-IntegrationTests.ps1
```

## Repository Folders

1. "src" contains the application screens and workflows.
2. "src-tauri" contains the Rust code, receiver communication, and local data handling.
3. "src-tauri\resources" contains the starting application configuration files.
4. "services\receiver-registry" contains the central service, database updates, tests, and server scripts.

## Internal Use

QA Receiver Center and QA Receiver Registry are for the internal Trimble network only. The registry must not be available from the public internet.

# QA Receiver Center

QA Receiver Center is a Windows desktop application for monitoring and maintaining Trimble QA receiver fleets. It combines receiver inventory, diagnostics, firmware workflows, configuration backups, update history, and recovery tools in one interface.

Current application version: 0.1.0

## Main Capabilities

- Load the receiver inventory from the QA Azure DevOps repository with Windows authentication.
- Scan receiver availability, firmware, uptime, fix type, temperature, satellites, and error state.
- Run single, multiple, Alpha, and Beta firmware update workflows.
- Resolve firmware packages by receiver model from the internal release catalog.
- Upload local ".img" and ".timg" packages.
- Monitor receiver-side firmware progress and restore receiver passwords.
- Create and download configuration backups before firmware updates.
- Review and manage firmware update history.

## Technology

- React, TypeScript, and Vite
- Tauri 2 and Rust
- Trimble Modus Web Components
- Windows WebView2

## Workstation Requirements

- Node.js 22 LTS
- Rust toolchain installed with rustup
- Microsoft C++ Build Tools
- Microsoft Edge WebView2 Runtime
- Access to the internal Trimble network, Azure DevOps, and firmware release server

## First Start

### 1. Install Node.js 22 LTS

1. Open https://nodejs.org/en/download.
2. Download the Windows Installer for Node.js 22 LTS.
3. Run the installer and keep the default options enabled.
4. Open a new Command Prompt and verify the installation:

~~~powershell
node --version
npm --version
~~~

### 2. Install Rust

1. Open https://rustup.rs.
2. Download and run "rustup-init.exe".
3. Select the default installation option.
4. If the installer requests Microsoft C++ Build Tools, install the "Desktop development with C++" workload.
5. Open a new Command Prompt and verify the installation:

~~~powershell
rustc --version
cargo --version
~~~

### 3. Start QA Receiver Center

Double-click "start-tauri-dev.bat" in the repository folder.

The launcher installs JavaScript dependencies automatically on the first start and then opens the Tauri application. The first Rust build downloads and compiles the required packages, so it can take several minutes. Keep the launcher window open until the application appears.

No local password file or password configuration is required.

The launcher also applies a process-local Cargo compatibility setting for Trimble workstations where the corporate network cannot reach the Windows certificate revocation service. It does not change the global Cargo or Windows configuration, and regular TLS certificate validation remains enabled.

The Vite development server ignores the Rust build directory. Cargo monitors Rust source changes independently, which prevents Windows file-lock errors while Rust executables are being compiled.

## Optional Environment Overrides

The default editable firmware configuration is created on first use at:

"%APPDATA%\QA Receiver Center\firmware-configs.json"

The bundled file in "src-tauri/resources/firmware-configs.json" is used only as the initial template.

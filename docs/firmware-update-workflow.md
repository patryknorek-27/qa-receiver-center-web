# Firmware Update Workflow

This document defines the safe firmware update flow for QA Receiver Center.
It is based on the existing Python updater, but the new implementation should be
state-driven, observable from the UI, and conservative around credential restore.

## Source Inputs

- Receiver credentials: `qa-receivers.json`
- Firmware group membership and receiver model mapping: `configs.json`
- Firmware release source: Trimble release HTTP server
- Target package: derived from firmware version, receiver model, and suffix rules

## Safety Rules

1. Never start upload if the target firmware package is unavailable.
2. Never update a receiver that is not explicitly assigned to the selected firmware config group.
3. Never perform real upload before credentials are verified.
4. If a temporary password was applied, restoring the original receiver password has highest priority.
5. A failed upload, timeout, connection drop, or user stop must still attempt password restore.
6. UI status must be driven by backend state, not guessed by the frontend.
7. Group updates must be built on top of the single receiver workflow.

## Single Receiver State Machine

| State | Purpose | Next Success | Next Failure |
| --- | --- | --- | --- |
| `Queued` | Receiver is selected and waiting. | `Preflight` | `Failed` |
| `Preflight` | Verify credentials, config membership, package target, and current install state. | `PackageCheck` | `Failed` |
| `PackageCheck` | Confirm firmware file exists remotely or locally. | `DownloadPackage` or `ChangePassword` | `Failed` |
| `DownloadPackage` | Download firmware package if not cached. | `ChangePassword` | `Failed` |
| `ChangePassword` | Change receiver password to temporary update password. | `UploadFirmware` | `Failed` |
| `UploadFirmware` | POST firmware package to receiver firmware endpoint. | `WaitForInstall` | `RestorePassword` |
| `WaitForInstall` | Wait through reboot and install marker checks. | `RestorePassword` | `RestorePassword` |
| `RestorePassword` | Restore original password from receiver config. | `VerifyResult` | `RestoreFailed` |
| `VerifyResult` | Confirm receiver is reachable and firmware status is readable. | `Completed` | `Failed` |
| `Completed` | Update finished successfully. | terminal | terminal |
| `Failed` | Update failed before or after restore was not needed. | terminal | terminal |
| `RestoreFailed` | Update path failed and password restore also failed. | terminal | terminal |
| `Stopped` | User requested stop before unsafe step or between receivers. | terminal | terminal |

## UI Status Mapping

The frontend should display the backend state per receiver:

- `Queued`
- `Preflight`
- `Package Check`
- `Downloading`
- `Changing Password`
- `Uploading`
- `Installing`
- `Restoring Password`
- `Verifying`
- `Completed`
- `Failed`
- `Restore Failed`
- `Stopped`

## Backend Phases

### Phase 1: Dry Run

Implement the full state machine without changing receiver state:

- Resolve receiver credentials.
- Resolve model and target package.
- Check whether firmware package is reachable.
- Check whether receiver is reachable.
- Detect whether receiver is already installing firmware.
- Emit state updates to UI.
- Do not change password.
- Do not upload firmware.

### Phase 2: Single Receiver Real Update

Enable real actions only for one receiver:

- Download package.
- Change password.
- Upload firmware.
- Wait for install/reboot.
- Restore password.
- Verify receiver status.

### Phase 3: Group Queue

Use the same single receiver workflow for Alpha/Beta groups:

- Sequential mode first.
- Parallel mode later with a strict worker limit.
- Per receiver cancellation checkpoints.
- Global stop should prevent new unsafe steps but still allow password restore.

## Notes From Old Python Updater

- Firmware package URLs are built from release version, model name, and suffix.
- The installer filename is built as `model + suffix`, for example `argon_titan_V700_034.timg`.
- Some models can have model-specific suffixes.
- Firmware availability is checked before changing any receiver.
- Firmware upload may end with timeout/connection drop while the receiver continues installing.
- Install status is inferred by checking the receiver web interface for known install markers.
- Reboot/install wait can take a long time and must not block the UI thread.
- Password restore is attempted both on normal path and exception path.

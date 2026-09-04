# Password recovery fix

## Problem

The old password check read "merge.xml?sysData=" and treated any successful HTTP response as a successful login. A receiver with anonymous access can return this data without accepting the password. This could skip the password change and report that recovery was complete while the temporary password was still active.

The old recovery loop could retry rejected credentials 12 times. Firmware monitoring also switched between temporary and configured passwords after failed requests. A failed update step could stop UI polling before password recovery finished.

The first fix required an HTTP 401 challenge from "prog/show?serialNumber". This was also incorrect: this command is public on the reported BD940-INS receivers. It blocked valid credentials, and the same check could prevent recovery after a password change. A public read is neither a successful login nor a rejected password.

Trimble documents [anonymous access and failed login lockouts](https://help.fieldsystems.trimble.com/alloy/help-security-config.htm).

## Receiver protocol checked on 2026-09-03

The public interface files on "10.2.155.135" and "10.2.155.59" were read without credentials or receiver changes. Both report "BD940-INS", firmware "6.63-0.73", and anonymous access enabled.

Their "CACHEDIR7043001866/xml/login.html" submits "username", "password" and "t" as a form POST to "cgi-bin/login.xml". The time field counts seconds from 1980-01-06. The page handles "FAIL" as rejected credentials, "LOCKED" as a blocked account, and a "data" response containing "cookie" and "serial" as successful login. These XML results matter even when the HTTP status is 200.

The anonymous "xml/dynamic/merge.xml?sysData=" response reports an empty "SecUser" and "LoggedIn" equal to zero. It can only be used to check that the web interface is ready. Test response examples use this observed structure with fake serial numbers and session tokens, not saved credentials.

## Changes

1. Check the configured password before preflight and Clone File operations, using the receiver's actual login form. Record a separate "Check Password" step. A failure here explicitly states that this job sent no password change or firmware upload. A public status response does not block valid credentials. Login is confirmed only by the session response from "login.xml". The session token is never saved, logged or reused for later password checks.
2. Start recovery protection before sending the temporary password change. A lost response can hide a successful change.
3. Restore using the expected temporary password, then confirm the configured password with at most one fresh login attempt. This login is outside every retry loop, including on connection errors, lost responses and receiver errors. Only anonymous readiness reads may repeat while waiting for the receiver. A failed or uncertain confirmation stops recovery with a visible warning. Do not repeat a password change when its result is unclear. The separate temporary password check precedes the write. If that check rejects the temporary password, check the configured password once to detect an already restored receiver, without sending a change. Never cycle through possible passwords. These limits do not control login attempts from other applications.
4. Keep the job active until recovery finishes. A recovery failure overrides a normal completion or cancellation result and is saved in update history. Single, multiple and group updates use the same worker. The older direct update command now uses that worker too.
5. Save a local recovery record before changing credentials. The warning survives an application restart, contains no passwords, and disappears only after verified recovery. An unreadable record blocks new updates. Do not manually delete the record to hide a warning.
6. Show the warning on every application page. Block another update of an affected receiver and pause its background scans. Receiver operation locks prevent scans and password changes from running at the same time within this application process.
7. Prevent manual password recovery from overlapping an active firmware job. Firmware monitoring stops on authentication rejection instead of repeatedly trying different passwords.
8. Distinguish a failed initial check from a failed login after the temporary password change was acknowledged. Check "LOCKED" and "FAIL" XML responses during password changes too, so a lockout is not hidden behind HTTP 200.
9. Open one shared warning dialog for unresolved password recovery incidents. Show the IP, receiver name and model when available, and the failure detail from the completed job. Other updates continue. "Understood", the close button and Escape only acknowledge the visible warning; they do not clear its record, contact a receiver or start another login. "View details" reopens the same information without another sound. Unresolved warnings return after an application restart.
10. Request one Windows warning sound when new incidents arrive. Failures arriving in the same poll share one sound. Both the interface and the native command track incident identities using IP and job ID, so polling or reopening the dialog cannot repeat the sound. Audio errors do not suppress the dialog. Playback uses [Windows MessageBeep](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-messagebeep), so Windows sound settings and output volume still apply. The application cannot guarantee audibility on a muted computer.

## Local checks

The automated tests use fake receivers on "127.0.0.1". They do not contact lab receivers or the registry server.

They cover anonymous and protected status pages, successful form login, rejected credentials and account lockouts inside HTTP 200, form encoding, fresh sessions, false change acknowledgements, lost replies, recovery after a failed activation check, and the shared initial check for single, multiple and group jobs. Request counters enforce exactly one configured password confirmation after restoration, including failure cases. Connection errors during login are terminal; readiness retries send no credentials.

The separate "approved_receiver_login_only" test is ignored by default. It requires the operator's explicit approval, one IP in "QA_APPROVED_LOGIN_CHECK_IP", and an explicit test selection. It calls the production verifier once, reads credentials from the normal configuration, and cannot change passwords or upload firmware. Do not include ignored tests in unattended test runs.

On 2026-09-03 the operator approved one login check on "10.2.155.135". The production verifier accepted the configured password. The check sent no password change, reboot or firmware upload. No login check was performed on "10.2.155.59". This confirms the login protocol on the first receiver, not a complete firmware update and recovery cycle. An earlier test run stopped while reading network configuration, before contacting the receiver for login.

```powershell
cd src-tauri
cargo test --lib --offline
cd ..
npm.cmd run lint
npm.cmd run build
node --test tests/password-recovery-notifications.test.ts
```

The test preview at "/tests/password-recovery-preview.html" uses Tauri's mock API and blocks all real native operations, including sound playback. It is not part of the production build. The "scenario=error", "scenario=resolved", "scenario=multiple", "scenario=many" and "scenario=sound-error" query options cover missing records, cleared warnings, grouped warnings, scrolling and unavailable audio. The "scenario=interactive" option adds controls for new incidents and resolution, with counters for sound requests and blocked native operations. It expects the local preview server on port 5174.

The separate ignored "windows_warning_sound_is_available" test plays one system sound and contacts no receiver. On 2026-09-03 Windows accepted that playback request. This confirms the API call, not that the user heard the sound.

## Controlled receiver test

1. Do not resume group updates with the old application. First unlock affected receivers and confirm their configured passwords manually. Do not guess passwords or repeat failed logins.
2. Pause external restart and monitoring scripts for one selected test receiver. Keep access to its normal management interface and configuration backup. These local locks do not control other applications, computers or scripts.
3. Run the updated desktop application from this source. Do not assume an older installed executable contains the fix. No registry server update is needed for this change.
4. First confirm one login with the configured password, without changing the password or starting firmware installation. A successful login checks the verifier only; it is not a full firmware acceptance test.
5. Then test one firmware update. Confirm that "Restore Password" completes, the job finishes, and the configured account works after the receiver has restarted. Check the history entry and Clone File. Other models or firmware versions may need their login protocol checked before use. Do not disable receiver security to bypass an error.
6. If recovery cannot be confirmed, stop further updates. Keep the warning, note the receiver IP and failed step, and check its password manually. Use "Restore Password" only when no update is running. Re-enable external scripts only after successful verification.
7. After the single receiver test passes, test a small group before resuming normal group updates.

The local tests demonstrate the corrected logic. They do not prove compatibility with every receiver model or firmware version. A real receiver acceptance test is still required. Recovery records created by this fix cannot identify receivers left with a temporary password by an older application version.

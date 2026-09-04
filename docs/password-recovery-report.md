# Password recovery and user warning

The application restores the password loaded for the receiver IP from "qa-receivers.json". After the receiver is ready, it makes at most one login attempt to confirm that password. A rejected login, timeout or connection error does not trigger another confirmation attempt.

## Single confirmation attempt

Source: "src-tauri/src/receiver_password.rs", function "verify_when_ready".

```rust
let client = client()?;
let serial = wait_until_ready(&client, rx, delay)?;
login_once(&client, rx, password, serial.as_deref())
```

Only "wait_until_ready" may repeat requests. Those requests contain no credentials. "login_once" is outside that loop. The recovery function calls it with "rx.pw", the configured password, after the password change. Failure leaves the recovery warning active and prevents a successful update result.

## Visible warning and sound

Source: "src/PasswordRecoveryAlerts.tsx", new incident handling.

```tsx
const fresh = collectNewRecoveryNotices(result, seen.current)
if (fresh.length > 0) {
  setDialogOpen(true)
  setSoundError(false)
  void invoke('play_password_recovery_sound', {
    noticeIds: fresh.map(({ ip, jobId }) => ({ ip, jobId })),
  }).catch(() => { if (!cancelled) setSoundError(true) })
}
```

The dialog lists affected receivers in one place. The same IP and job ID do not trigger another sound on subsequent polls. A second check in "src-tauri/src/password_recovery_notices.rs", function "notify_once", also prevents duplicate playback. No passwords or session tokens are shown in the warning.

## Acknowledgement

Source: "src/PasswordRecoveryAlerts.tsx", function "acknowledge".

```tsx
setAcknowledged((current) => new Set([...current, ...notices.map(recoveryNoticeKey)]))
setDialogOpen(false)
```

Acknowledgement changes only the interface state. It does not retry login, clear the recovery record, cancel other updates or mark the receiver as recovered.

## Verification scope

Local tests check the single confirmation limit, grouped alerts, duplicate sound prevention and unavailable audio. Browser checks use mock receivers only. A previously approved single login on "10.2.155.135" succeeded. A complete firmware update and password recovery cycle on real hardware still requires a controlled acceptance test.

use super::qa_receiver_center_data_dir;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    path::Path,
    sync::{Mutex, OnceLock},
};

static STORE_LOCK: Mutex<()> = Mutex::new(());
static SOUNDED_NOTICES: OnceLock<Mutex<HashSet<PasswordRecoveryNoticeId>>> = OnceLock::new();

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(super) struct PasswordRecoveryNoticeId {
    pub ip: String,
    pub job_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PasswordRecoveryNotice {
    pub ip: String,
    pub job_id: String,
    pub message: String,
}

fn read_at(path: &Path) -> Result<Vec<PasswordRecoveryNotice>, String> {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).map_err(|_| {
            "Cannot read the password recovery record. Check it before updating receivers.".into()
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(_) => Err("Cannot open the password recovery record.".into()),
    }
}

pub(super) fn read() -> Result<Vec<PasswordRecoveryNotice>, String> {
    let _lock = STORE_LOCK
        .lock()
        .map_err(|_| "Password recovery record is unavailable.")?;
    read_at(&qa_receiver_center_data_dir()?.join("password-recovery.json"))
}

fn update(ip: &str, notice: Option<PasswordRecoveryNotice>) -> Result<(), String> {
    let _lock = STORE_LOCK
        .lock()
        .map_err(|_| "Password recovery record is unavailable.")?;
    let path = qa_receiver_center_data_dir()?.join("password-recovery.json");
    update_at(&path, ip, notice)
}

fn update_at(path: &Path, ip: &str, notice: Option<PasswordRecoveryNotice>) -> Result<(), String> {
    let mut notices = read_at(path)?;
    notices.retain(|entry| entry.ip != ip);
    if let Some(notice) = notice {
        notices.push(notice);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|_| "Cannot create the password recovery record directory.")?;
    }
    let bytes = serde_json::to_vec_pretty(&notices)
        .map_err(|_| "Cannot encode the password recovery record.")?;
    // Flush before any receiver password change. A corrupt record fails closed.
    let mut file =
        fs::File::create(&path).map_err(|_| "Cannot save the password recovery record.")?;
    std::io::Write::write_all(&mut file, &bytes)
        .map_err(|_| "Cannot save the password recovery record.")?;
    file.sync_all()
        .map_err(|_| "Cannot flush the password recovery record.".to_string())
}

pub(super) fn arm(ip: &str, job_id: &str) -> Result<(), String> {
    update(ip, Some(PasswordRecoveryNotice {
        ip: ip.to_string(),
        job_id: job_id.to_string(),
        message: "Configured password recovery has not been confirmed. Stop other scripts using this receiver and verify its password before another update.".into(),
    }))
}

pub(super) fn clear(ip: &str) -> Result<(), String> {
    update(ip, None)
}

pub(super) fn ensure_ready(ip: &str) -> Result<(), String> {
    if read()?.iter().any(|notice| notice.ip == ip) {
        return Err(format!("PASSWORD RECOVERY REQUIRED for {ip}. Resolve the previous password recovery warning before another update."));
    }
    Ok(())
}

pub(super) fn play_sound_once(
    requested: &[PasswordRecoveryNoticeId],
    current: &[PasswordRecoveryNoticeId],
) -> Result<bool, String> {
    let mut heard = SOUNDED_NOTICES
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .map_err(|_| "Warning sound state is unavailable.")?;
    notify_once(requested, current, &mut heard, play_warning_sound)
}

fn notify_once(
    requested: &[PasswordRecoveryNoticeId],
    current: &[PasswordRecoveryNoticeId],
    heard: &mut HashSet<PasswordRecoveryNoticeId>,
    play: impl FnOnce() -> Result<(), String>,
) -> Result<bool, String> {
    let fresh: Vec<_> = requested
        .iter()
        .filter(|id| current.contains(id) && !heard.contains(id))
        .cloned()
        .collect();
    if fresh.is_empty() {
        return Ok(false);
    }
    // Record delivery before playing: even an unavailable sound is not retried.
    heard.extend(fresh);
    play()?;
    Ok(true)
}

#[cfg(target_os = "windows")]
fn play_warning_sound() -> Result<(), String> {
    #[link(name = "user32")]
    extern "system" {
        fn MessageBeep(sound_type: u32) -> i32;
    }
    // SAFETY: MessageBeep takes a documented integer constant, with no pointers.
    if unsafe { MessageBeep(0x00000010) } == 0 {
        Err("Windows could not play the warning sound.".into())
    } else {
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
fn play_warning_sound() -> Result<(), String> {
    Err("The warning sound is only available in the Windows application.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notice_id(ip: &str, job_id: &str) -> PasswordRecoveryNoticeId {
        PasswordRecoveryNoticeId {
            ip: ip.into(),
            job_id: job_id.into(),
        }
    }

    #[test]
    fn sound_is_once_per_new_incident_and_ignores_unknown_or_resolved_ids() {
        let first = notice_id("192.0.2.10", "group-a");
        let second = notice_id("192.0.2.11", "group-a");
        let repeat_ip = notice_id("192.0.2.10", "group-b");
        let mut heard = HashSet::new();
        let mut calls = 0;
        let current = vec![first.clone(), second.clone()];
        for _ in 0..5 {
            notify_once(&current, &current, &mut heard, || {
                calls += 1;
                Ok(())
            })
            .unwrap();
        }
        assert_eq!(calls, 1);
        assert!(!notify_once(
            std::slice::from_ref(&repeat_ip),
            &current,
            &mut heard,
            || panic!("Unknown incident")
        )
        .unwrap());
        assert!(!notify_once(&current, &[], &mut heard, || panic!("Resolved incident")).unwrap());
        notify_once(
            std::slice::from_ref(&repeat_ip),
            std::slice::from_ref(&repeat_ip),
            &mut heard,
            || {
                calls += 1;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(calls, 2);
    }

    #[test]
    fn unavailable_sound_is_not_retried() {
        let current = vec![notice_id("192.0.2.10", "test")];
        let mut heard = HashSet::new();
        assert!(notify_once(&current, &current, &mut heard, || Err("Muted".into())).is_err());
        assert!(!notify_once(&current, &current, &mut heard, || panic!(
            "Sound was retried"
        ))
        .unwrap());
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "Plays one Windows warning sound; run explicitly for audio verification"]
    fn windows_warning_sound_is_available() {
        play_warning_sound().unwrap();
    }

    #[test]
    fn recovery_record_contains_no_credentials() {
        let notice = PasswordRecoveryNotice {
            ip: "127.0.0.1".into(),
            job_id: "test".into(),
            message: "Recovery pending".into(),
        };
        let json = serde_json::to_value(notice).unwrap();
        assert_eq!(json.as_object().unwrap().len(), 3);
        assert!(json.get("password").is_none());
    }

    #[test]
    fn pending_recovery_survives_reload_and_clearing_one_receiver_preserves_others() {
        let path = std::env::temp_dir().join(format!(
            "qa-password-recovery-test-{}-{}.json",
            std::process::id(),
            crate::now_millis()
        ));
        for ip in ["192.0.2.10", "192.0.2.11"] {
            update_at(
                &path,
                ip,
                Some(PasswordRecoveryNotice {
                    ip: ip.into(),
                    job_id: "test".into(),
                    message: "Pending".into(),
                }),
            )
            .unwrap();
        }
        assert_eq!(read_at(&path).unwrap().len(), 2);
        update_at(&path, "192.0.2.10", None).unwrap();
        let reloaded = read_at(&path).unwrap();
        assert_eq!(reloaded.len(), 1);
        assert_eq!(reloaded[0].ip, "192.0.2.11");
        fs::write(&path, "{broken").unwrap();
        assert!(read_at(&path).is_err());
        assert!(update_at(&path, "192.0.2.11", None).is_err());
        fs::remove_file(path).unwrap();
    }
}

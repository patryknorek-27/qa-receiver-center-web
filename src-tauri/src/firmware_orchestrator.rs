use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub enum FirmwareJobKind {
    Single,
    Multiple,
    Group,
}

#[derive(Clone, Debug)]
pub struct FirmwareJobReservation {
    pub job_id: String,
    pub kind: FirmwareJobKind,
    pub receiver_ips: Vec<String>,
    pub max_parallel_receivers: usize,
}

impl FirmwareJobReservation {
    pub fn single(job_id: impl Into<String>, receiver_ip: impl Into<String>) -> Self {
        Self {
            job_id: job_id.into(),
            kind: FirmwareJobKind::Single,
            receiver_ips: vec![receiver_ip.into()],
            max_parallel_receivers: 1,
        }
    }
}

#[derive(Clone, Default)]
pub struct FirmwareOrchestrator {
    inner: Arc<Mutex<OrchestratorState>>,
}

#[derive(Default)]
struct OrchestratorState {
    jobs: HashMap<String, ActiveFirmwareJob>,
    receiver_owners: HashMap<String, String>,
}

struct ActiveFirmwareJob {
    kind: FirmwareJobKind,
    receiver_ips: Vec<String>,
    cancellation: Arc<AtomicBool>,
}

impl FirmwareOrchestrator {
    pub fn reserve(
        &self,
        mut reservation: FirmwareJobReservation,
        max_active_jobs_for_kind: usize,
    ) -> Result<FirmwareJobLease, String> {
        reservation.job_id = reservation.job_id.trim().to_string();
        if reservation.job_id.is_empty() {
            return Err("Firmware job ID is required.".to_string());
        }
        if reservation.max_parallel_receivers == 0 {
            return Err("Firmware job concurrency must be at least one.".to_string());
        }

        let mut seen = HashSet::new();
        reservation.receiver_ips = reservation
            .receiver_ips
            .into_iter()
            .map(|ip| ip.trim().to_string())
            .filter(|ip| !ip.is_empty() && seen.insert(ip.clone()))
            .collect();
        if reservation.receiver_ips.is_empty() {
            return Err("Select at least one receiver for the firmware job.".to_string());
        }

        let mut state = self
            .inner
            .lock()
            .map_err(|_| "Firmware orchestrator is unavailable.".to_string())?;
        if state.jobs.contains_key(&reservation.job_id) {
            return Err(format!(
                "Firmware job {} is already registered.",
                reservation.job_id
            ));
        }

        if reservation.kind == FirmwareJobKind::Group && !state.jobs.is_empty() {
            return Err(
                "A group update can start only when no other firmware updates are running."
                    .to_string(),
            );
        }
        if reservation.kind != FirmwareJobKind::Group
            && state
                .jobs
                .values()
                .any(|job| job.kind == FirmwareJobKind::Group)
        {
            return Err("A group firmware update is already running.".to_string());
        }

        let active_for_kind = state
            .jobs
            .values()
            .filter(|job| job.kind == reservation.kind)
            .count();
        if active_for_kind >= max_active_jobs_for_kind.max(1) {
            return Err(match reservation.kind {
                FirmwareJobKind::Single => format!(
                    "Maximum {} single receiver updates can run at the same time.",
                    max_active_jobs_for_kind.max(1)
                ),
                FirmwareJobKind::Multiple => {
                    "Another multiple receiver update session is already running.".to_string()
                }
                FirmwareJobKind::Group => {
                    "Another group firmware update session is already running.".to_string()
                }
            });
        }

        for ip in &reservation.receiver_ips {
            if let Some(owner) = state.receiver_owners.get(ip) {
                return Err(format!(
                    "Receiver {ip} already belongs to running firmware job {owner}."
                ));
            }
        }

        let cancellation = Arc::new(AtomicBool::new(false));
        for ip in &reservation.receiver_ips {
            state
                .receiver_owners
                .insert(ip.clone(), reservation.job_id.clone());
        }
        state.jobs.insert(
            reservation.job_id.clone(),
            ActiveFirmwareJob {
                kind: reservation.kind,
                receiver_ips: reservation.receiver_ips,
                cancellation: Arc::clone(&cancellation),
            },
        );

        Ok(FirmwareJobLease {
            orchestrator: self.clone(),
            job_id: reservation.job_id,
            cancellation,
            max_parallel_receivers: reservation.max_parallel_receivers,
            released: false,
        })
    }

    pub fn request_cancel(&self, job_id: &str) -> Result<(), String> {
        let state = self
            .inner
            .lock()
            .map_err(|_| "Firmware orchestrator is unavailable.".to_string())?;
        let job = state
            .jobs
            .get(job_id)
            .ok_or_else(|| format!("Active firmware job not found: {job_id}"))?;
        job.cancellation.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn release(&self, job_id: &str) {
        if let Ok(mut state) = self.inner.lock() {
            if let Some(job) = state.jobs.remove(job_id) {
                for ip in job.receiver_ips {
                    if state.receiver_owners.get(&ip).map(String::as_str) == Some(job_id) {
                        state.receiver_owners.remove(&ip);
                    }
                }
            }
        }
    }

    #[cfg(test)]
    fn active_job_count(&self, kind: FirmwareJobKind) -> usize {
        self.inner
            .lock()
            .map(|state| state.jobs.values().filter(|job| job.kind == kind).count())
            .unwrap_or_default()
    }
}

pub struct FirmwareJobLease {
    orchestrator: FirmwareOrchestrator,
    job_id: String,
    cancellation: Arc<AtomicBool>,
    max_parallel_receivers: usize,
    released: bool,
}

impl FirmwareJobLease {
    pub fn max_parallel_receivers(&self) -> usize {
        self.max_parallel_receivers
    }

    pub fn control(&self) -> FirmwareJobControl {
        FirmwareJobControl {
            cancellation: Arc::clone(&self.cancellation),
        }
    }
}

#[derive(Clone)]
pub struct FirmwareJobControl {
    cancellation: Arc<AtomicBool>,
}

impl FirmwareJobControl {
    pub fn is_cancel_requested(&self) -> bool {
        self.cancellation.load(Ordering::SeqCst)
    }
}

impl Drop for FirmwareJobLease {
    fn drop(&mut self) {
        if !self.released {
            self.orchestrator.release(&self.job_id);
            self.released = true;
        }
    }
}

pub struct PasswordRecoveryGuard<F>
where
    F: FnOnce() -> Result<(), String>,
{
    recovery: Option<F>,
}

impl<F> PasswordRecoveryGuard<F>
where
    F: FnOnce() -> Result<(), String>,
{
    pub fn new(recovery: F) -> Self {
        Self {
            recovery: Some(recovery),
        }
    }

    pub fn restore_now(&mut self) -> Result<(), String> {
        match self.recovery.take() {
            Some(recovery) => recovery(),
            None => Ok(()),
        }
    }
}

impl<F> Drop for PasswordRecoveryGuard<F>
where
    F: FnOnce() -> Result<(), String>,
{
    fn drop(&mut self) {
        if let Some(recovery) = self.recovery.take() {
            let _ = recovery();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn receiver_cannot_be_reserved_by_two_jobs() {
        let orchestrator = FirmwareOrchestrator::default();
        let first = orchestrator
            .reserve(FirmwareJobReservation::single("job-1", "10.2.155.10"), 3)
            .expect("first reservation should succeed");

        let second =
            orchestrator.reserve(FirmwareJobReservation::single("job-2", "10.2.155.10"), 3);

        assert!(second.is_err());
        drop(first);
        assert!(orchestrator
            .reserve(FirmwareJobReservation::single("job-3", "10.2.155.10"), 3)
            .is_ok());
    }

    #[test]
    fn job_kind_limit_is_enforced_and_lease_releases_capacity() {
        let orchestrator = FirmwareOrchestrator::default();
        let first = orchestrator
            .reserve(FirmwareJobReservation::single("job-1", "10.2.155.10"), 1)
            .expect("first reservation should succeed");

        assert!(orchestrator
            .reserve(FirmwareJobReservation::single("job-2", "10.2.155.11"), 1)
            .is_err());
        assert_eq!(orchestrator.active_job_count(FirmwareJobKind::Single), 1);
        assert_eq!(first.max_parallel_receivers(), 1);

        drop(first);
        assert_eq!(orchestrator.active_job_count(FirmwareJobKind::Single), 0);
    }

    #[test]
    fn cancellation_is_visible_to_running_job() {
        let orchestrator = FirmwareOrchestrator::default();
        let lease = orchestrator
            .reserve(FirmwareJobReservation::single("job-1", "10.2.155.10"), 3)
            .expect("reservation should succeed");

        orchestrator
            .request_cancel("job-1")
            .expect("cancellation should succeed");
        assert!(lease.control().is_cancel_requested());
    }

    #[test]
    fn cancelled_job_releases_receiver_for_the_next_update() {
        let orchestrator = FirmwareOrchestrator::default();
        let lease = orchestrator
            .reserve(FirmwareJobReservation::single("job-1", "10.2.155.10"), 3)
            .expect("first update should reserve the receiver");

        orchestrator
            .request_cancel("job-1")
            .expect("cancellation should succeed");
        drop(lease);

        assert!(orchestrator
            .reserve(FirmwareJobReservation::single("job-2", "10.2.155.10"), 3)
            .is_ok());
    }

    #[test]
    fn password_recovery_runs_once_when_explicit_or_dropped() {
        let explicit_count = Arc::new(AtomicUsize::new(0));
        let explicit_counter = Arc::clone(&explicit_count);
        let mut explicit = PasswordRecoveryGuard::new(move || {
            explicit_counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        explicit
            .restore_now()
            .expect("explicit recovery should succeed");
        drop(explicit);
        assert_eq!(explicit_count.load(Ordering::SeqCst), 1);

        let drop_count = Arc::new(AtomicUsize::new(0));
        let drop_counter = Arc::clone(&drop_count);
        {
            let _guard = PasswordRecoveryGuard::new(move || {
                drop_counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            });
        }
        assert_eq!(drop_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn future_multiple_and_group_jobs_use_the_same_reservation_engine() {
        let orchestrator = FirmwareOrchestrator::default();
        let multiple = orchestrator
            .reserve(
                FirmwareJobReservation {
                    job_id: "multiple-1".to_string(),
                    kind: FirmwareJobKind::Multiple,
                    receiver_ips: vec!["10.2.155.10".to_string(), "10.2.155.11".to_string()],
                    max_parallel_receivers: 2,
                },
                1,
            )
            .expect("multiple reservation should succeed");

        assert!(orchestrator
            .reserve(
                FirmwareJobReservation {
                    job_id: "group-1".to_string(),
                    kind: FirmwareJobKind::Group,
                    receiver_ips: vec!["10.2.155.11".to_string()],
                    max_parallel_receivers: 1,
                },
                1,
            )
            .is_err());

        drop(multiple);
        assert!(orchestrator
            .reserve(
                FirmwareJobReservation {
                    job_id: "group-2".to_string(),
                    kind: FirmwareJobKind::Group,
                    receiver_ips: vec!["10.2.155.11".to_string()],
                    max_parallel_receivers: 1,
                },
                1,
            )
            .is_ok());
    }

    #[test]
    fn group_job_is_exclusive_across_firmware_operations() {
        let orchestrator = FirmwareOrchestrator::default();
        let single = orchestrator
            .reserve(FirmwareJobReservation::single("single-1", "10.2.155.10"), 3)
            .expect("single reservation should succeed");

        assert!(orchestrator
            .reserve(
                FirmwareJobReservation {
                    job_id: "group-1".to_string(),
                    kind: FirmwareJobKind::Group,
                    receiver_ips: vec!["10.2.155.20".to_string()],
                    max_parallel_receivers: 1,
                },
                1,
            )
            .is_err());

        drop(single);
        let group = orchestrator
            .reserve(
                FirmwareJobReservation {
                    job_id: "group-2".to_string(),
                    kind: FirmwareJobKind::Group,
                    receiver_ips: vec!["10.2.155.20".to_string()],
                    max_parallel_receivers: 1,
                },
                1,
            )
            .expect("group reservation should succeed");

        assert!(orchestrator
            .reserve(FirmwareJobReservation::single("single-2", "10.2.155.30"), 3)
            .is_err());
        assert!(orchestrator
            .reserve(
                FirmwareJobReservation {
                    job_id: "multiple-1".to_string(),
                    kind: FirmwareJobKind::Multiple,
                    receiver_ips: vec!["10.2.155.31".to_string()],
                    max_parallel_receivers: 1,
                },
                usize::MAX,
            )
            .is_err());
        drop(group);
    }
}

mod firmware_orchestrator;
mod password_recovery_notices;
mod receiver_password;

use firmware_orchestrator::{
    FirmwareJobControl, FirmwareJobKind, FirmwareJobLease, FirmwareJobReservation,
    FirmwareOrchestrator, PasswordRecoveryGuard,
};
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_TYPE, EXPECT};
use reqwest::{Method, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::fs;
use std::net::Ipv4Addr;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{webview::PageLoadEvent, State};

const DEFAULT_WORKERS: usize = 12;
const DEFAULT_INTERFACE_ZOOM: f64 = 0.94;
const MAX_SINGLE_FIRMWARE_JOBS: usize = 3;
const MAX_GROUP_FIRMWARE_JOBS: usize = 1;
const MAX_PARALLEL_FIRMWARE_RECEIVERS: usize = 25;
const MAX_PASSWORD_RECOVERY_PARALLEL_RECEIVERS: usize = 3;
const CONVERGENCE_RESTART_CYCLE_MINUTES: u32 = 30;
const DEFAULT_POST_RESTART_DELAY_MINUTES: u32 = 2;
const DEFAULT_PRE_RESTART_SAFETY_MARGIN_MINUTES: u32 = 10;
const APPLICATION_ADMIN_USERNAME: &str = "admin123";
const APPLICATION_ADMIN_PASSWORD: &str = "123";
const FIRMWARE_TEMP_PASSWORD: &str = "fwreboot123";
const FAST_IMAGE_INSTALL_START_PROGRESS: f64 = 15.0;
const FAST_IMAGE_INSTALL_COMPLETE_PROGRESS: f64 = 72.0;
const FAST_IMAGE_RESTART_PROGRESS: f64 = 82.0;
const FAST_IMAGE_ONLINE_PROGRESS: f64 = 90.0;
const FAST_IMAGE_PASSWORD_RESTORED_PROGRESS: f64 = 96.0;
const FAST_IMAGE_VERIFY_PROGRESS: f64 = 98.0;
const GROUPS_JSON_PATH: &str =
    r"\\det-fileserver\TeamBuilds\QA\rx-monitoring-dhs\main\qa-receivers.json";
const GROUPS_ADO_API_URL: &str = "https://det-ados/tfs/Terrasat/QA/_apis/git/repositories/rx-monitoring-dhs/items?path=%2Fqa-receivers.json&versionDescriptor.version=main&versionDescriptor.versionType=branch&includeContent=true&api-version=7.0";
const RECEIVER_REGISTRY_BASE_URL: &str = "http://HHKNQA1-DE-DE04.eu.trimblecorp.net:8787";
const FIRMWARE_RELEASE_BASE_URL: &str = "http://higgs.eng.trimble.com/data_drive/WWW/Releases";
const DEFAULT_FIRMWARE_CONFIG: &str = include_str!("../resources/firmware-configs.json");
const INSTALL_MARKERS: [&str; 2] = [
    "Installing firmware from failsafe region",
    "Installing failsafe copy of firmware",
];

fn receiver_concurrency(parallel: bool, receiver_count: usize) -> usize {
    if parallel {
        receiver_count.clamp(1, MAX_PARALLEL_FIRMWARE_RECEIVERS)
    } else {
        1
    }
}

fn single_or_multiple_reservation(
    job_id: &str,
    receiver_ip: &str,
    batch_id: Option<&str>,
) -> (FirmwareJobReservation, usize) {
    if batch_id.is_some_and(|value| value.starts_with("multiple-fw-")) {
        (
            FirmwareJobReservation {
                job_id: job_id.to_string(),
                kind: FirmwareJobKind::Multiple,
                receiver_ips: vec![receiver_ip.to_string()],
                max_parallel_receivers: 1,
            },
            usize::MAX,
        )
    } else {
        (
            FirmwareJobReservation::single(job_id, receiver_ip),
            MAX_SINGLE_FIRMWARE_JOBS,
        )
    }
}

#[derive(Clone)]
struct ReceiverCredential {
    addr: String,
    user: String,
    pw: String,
    group: u8,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReceiverSummary {
    ip: String,
    group: u8,
    status: String,
    receiver_name: Option<String>,
    firmware_version: Option<String>,
    firmware_date: Option<String>,
    rtk_version: Option<String>,
    test_mode: Option<bool>,
    uptime_seconds: Option<i64>,
    celsius: Option<f64>,
    sv_solved: Option<f64>,
    fix_type: Option<String>,
    error_warning: Option<f64>,
    needs_review: bool,
    last_error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReceiverLoginDetails {
    ip: String,
    user: String,
    password: String,
}

#[derive(Default)]
struct FirmwareJobs {
    jobs: Arc<Mutex<HashMap<String, FirmwareJobStatus>>>,
    single_jobs: Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    orchestrator: FirmwareOrchestrator,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct FirmwareConfigMode {
    receiver_models: BTreeMap<String, String>,
    firmware_version: String,
    firmware_suffix: BTreeMap<String, String>,
    logo: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FirmwareConfigResponse {
    mode: String,
    group: u8,
    group_name: String,
    firmware_version: String,
    default_firmware_suffix: String,
    logo: String,
    receivers: Vec<FirmwareReceiver>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
struct UpdateSafetySettings {
    post_restart_delay_minutes: u32,
    pre_restart_safety_margin_minutes: u32,
    restart_cycle_minutes: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistrySettingEnvelope<T> {
    value: T,
    row_version: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RegistrySettingUpdate<'a, T> {
    value: &'a T,
    row_version: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct IpInventoryRecord {
    ip_address: String,
    sort_order: i32,
    status: String,
    location: Option<String>,
    responsible: Option<String>,
    environment: Option<String>,
    device_type: Option<String>,
    serial_number: Option<String>,
    antenna1: Option<String>,
    antenna2: Option<String>,
    notes: Option<String>,
    usage_end_date: Option<String>,
    is_online: Option<bool>,
    last_ping_at: Option<String>,
    row_version: i64,
    updated_by: String,
    updated_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct IpPingScanResult {
    total_count: i32,
    online_count: i32,
    offline_count: i32,
    started_at: String,
    completed_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct IpPingResult {
    ip_address: String,
    is_online: bool,
    checked_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct IpMonitoringSettings {
    automatic_scan_enabled: bool,
    interval_minutes: u32,
    timeout_milliseconds: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateIpInventoryRequest {
    status: String,
    location: Option<String>,
    responsible: Option<String>,
    environment: Option<String>,
    device_type: Option<String>,
    serial_number: Option<String>,
    antenna1: Option<String>,
    antenna2: Option<String>,
    notes: Option<String>,
    usage_end_date: Option<String>,
    row_version: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct IpRequestRecord {
    id: String,
    ip_address: String,
    requested_by: String,
    location: Option<String>,
    responsible: Option<String>,
    environment: Option<String>,
    device_type: Option<String>,
    serial_number: Option<String>,
    antenna1: Option<String>,
    antenna2: Option<String>,
    notes: Option<String>,
    usage_end_date: Option<String>,
    status: String,
    decision_by: Option<String>,
    decision_reason: Option<String>,
    decided_at: Option<String>,
    row_version: i64,
    created_at: String,
    updated_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApproveIpRequestRequest {
    location: Option<String>,
    responsible: Option<String>,
    environment: Option<String>,
    device_type: Option<String>,
    serial_number: Option<String>,
    antenna1: Option<String>,
    antenna2: Option<String>,
    notes: Option<String>,
    usage_end_date: Option<String>,
    request_row_version: i64,
    ip_row_version: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RejectIpRequestRequest {
    reason: String,
    row_version: i64,
}

impl Default for UpdateSafetySettings {
    fn default() -> Self {
        Self {
            post_restart_delay_minutes: DEFAULT_POST_RESTART_DELAY_MINUTES,
            pre_restart_safety_margin_minutes: DEFAULT_PRE_RESTART_SAFETY_MARGIN_MINUTES,
            restart_cycle_minutes: CONVERGENCE_RESTART_CYCLE_MINUTES,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateSafetyEvaluation {
    ip: String,
    allowed: bool,
    state: String,
    message: String,
    uptime_seconds: i64,
    available_in_seconds: Option<i64>,
    window_closes_in_seconds: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FirmwareReleaseCatalog {
    source_url: String,
    refreshed_at: String,
    releases: Vec<FirmwareReleaseSummary>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FirmwareReleaseSummary {
    id: String,
    name: String,
    url: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FirmwareBuildCatalog {
    release_id: String,
    release_name: String,
    source_url: String,
    refreshed_at: String,
    builds: Vec<FirmwareBuildSummary>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FirmwareBuildSummary {
    id: String,
    name: String,
    url: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FirmwareReleasePackage {
    file_name: String,
    url: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FirmwareReleaseModelMatch {
    model: String,
    status: String,
    selected_file: Option<String>,
    candidates: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FirmwareReleaseInspection {
    release_id: String,
    release_name: String,
    build_id: String,
    build_name: String,
    source_url: String,
    refreshed_at: String,
    packages: Vec<FirmwareReleasePackage>,
    model_matches: Vec<FirmwareReleaseModelMatch>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FirmwareReceiver {
    ip: String,
    model: String,
    group: u8,
    group_name: String,
    has_credentials: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FirmwarePlanRequest {
    mode: String,
    firmware_version: String,
    firmware_suffix: String,
    receiver_ips: Vec<String>,
    parallel: bool,
    #[serde(default)]
    package_overrides: Option<BTreeMap<String, String>>,
    #[serde(default)]
    local_package_paths: Option<BTreeMap<String, String>>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FirmwareSingleUpdateRequest {
    mode: String,
    batch_id: Option<String>,
    receiver_ip: String,
    receiver_model: Option<String>,
    firmware_version: String,
    firmware_suffix: String,
    #[serde(default)]
    package_file: Option<String>,
    source_mode: Option<String>,
    local_file_name: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FirmwareSingleLocalUpdateRequest {
    batch_id: Option<String>,
    receiver_ip: String,
    receiver_model: Option<String>,
    local_file_name: String,
    local_file_path: String,
}

#[derive(Clone)]
struct FirmwarePlanReceiver {
    credential: ReceiverCredential,
    model: String,
    target_file: String,
    target_url: String,
    local_package_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FirmwareWorkflowStep {
    state: String,
    status: String,
    message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FirmwareSingleDryRunResult {
    ip: String,
    model: String,
    target_file: String,
    target_url: String,
    receiver_name: Option<String>,
    firmware_version: Option<String>,
    uptime_seconds: Option<i64>,
    status: String,
    steps: Vec<FirmwareWorkflowStep>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FirmwareSingleUpdateJobStatus {
    job_id: String,
    batch_id: Option<String>,
    operation_type: Option<String>,
    group_name: Option<String>,
    ip: String,
    model: String,
    target_file: String,
    status: String,
    phase: String,
    message: String,
    receiver_name: Option<String>,
    firmware_version: Option<String>,
    uptime_seconds: Option<i64>,
    firmware_status: Option<String>,
    fw_status: Option<String>,
    update_mode: Option<String>,
    update_percent: Option<f64>,
    update_stage: Option<u8>,
    update_stage_percent: Option<f64>,
    update_in_progress: bool,
    downgrade_required: bool,
    downgrade_decision: Option<bool>,
    active_checksum: Option<String>,
    last_seen_at: Option<String>,
    clone_file_name: Option<String>,
    clone_file_path: Option<String>,
    started_at: String,
    completed_at: Option<String>,
    steps: Vec<FirmwareWorkflowStep>,
}

#[derive(Clone, Debug)]
struct CloneFileBackup {
    file_name: String,
    local_path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct FirmwareSingleUpdateHistoryEntry {
    job_id: String,
    batch_id: Option<String>,
    #[serde(default)]
    operation_type: Option<String>,
    #[serde(default)]
    group_name: Option<String>,
    ip: String,
    receiver_name: Option<String>,
    model: String,
    target_file: String,
    started_at: String,
    completed_at: Option<String>,
    result: String,
    final_firmware_version: Option<String>,
    clone_file_name: Option<String>,
    clone_file_path: Option<String>,
    errors: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistryCloneFileSummary {
    file_name: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistryFirmwareHistoryEntry {
    job_id: String,
    batch_id: Option<String>,
    operation_type: Option<String>,
    group_name: Option<String>,
    ip: String,
    receiver_name: Option<String>,
    model: String,
    target_file: String,
    started_at: String,
    completed_at: Option<String>,
    result: String,
    final_firmware_version: Option<String>,
    clone_file: Option<RegistryCloneFileSummary>,
    errors: Vec<String>,
}

impl RegistryFirmwareHistoryEntry {
    fn into_history_entry(self) -> FirmwareSingleUpdateHistoryEntry {
        let clone_file_name = self.clone_file.as_ref().map(|file| file.file_name.clone());
        let clone_file_path = self
            .clone_file
            .as_ref()
            .map(|_| format!("registry://firmware-history/{}", self.job_id));
        FirmwareSingleUpdateHistoryEntry {
            job_id: self.job_id,
            batch_id: self.batch_id,
            operation_type: self.operation_type,
            group_name: self.group_name,
            ip: self.ip,
            receiver_name: self.receiver_name,
            model: self.model,
            target_file: self.target_file,
            started_at: self.started_at,
            completed_at: self.completed_at,
            result: self.result,
            final_firmware_version: self.final_firmware_version,
            clone_file_name,
            clone_file_path,
            errors: self.errors,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PasswordRestoreResult {
    ip: String,
    status: String,
    message: String,
    restored_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FirmwarePasswordRecoveryBatchResult {
    job_id: String,
    status: String,
    total_count: usize,
    restored_count: usize,
    already_restored_count: usize,
    deferred_count: usize,
    failed_count: usize,
    receivers: Vec<PasswordRestoreResult>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CloneFileDownloadResult {
    file_name: String,
    destination_path: String,
}

#[derive(Clone, Debug)]
struct FirmwareReceiverLiveProbe {
    firmware_status: Option<String>,
    fw_status: Option<String>,
    update_mode: Option<String>,
    update_stage: Option<u8>,
    update_stage_percent: Option<f64>,
    update_in_progress: bool,
    active_checksum: Option<String>,
    receiver_name: Option<String>,
    firmware_version: Option<String>,
    uptime_seconds: Option<i64>,
}

#[derive(Clone, Copy, Debug)]
struct FirmwareInstallStage {
    index: u8,
    label: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalFirmwarePackage {
    name: String,
    path: String,
    size: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FirmwarePreflightResult {
    mode: String,
    group: u8,
    group_name: String,
    firmware_version: String,
    firmware_suffix: String,
    ready_count: usize,
    warning_count: usize,
    failed_count: usize,
    receivers: Vec<FirmwarePreflightReceiver>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FirmwarePreflightReceiver {
    ip: String,
    model: String,
    status: String,
    message: String,
    receiver_name: Option<String>,
    firmware_version: Option<String>,
    uptime_seconds: Option<i64>,
    target_file: String,
    target_url: String,
    can_update: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FirmwareJobStatus {
    job_id: String,
    mode: String,
    group: u8,
    group_name: String,
    firmware_version: String,
    firmware_suffix: String,
    parallel: bool,
    status: String,
    progress: f64,
    created_at: String,
    completed_at: Option<String>,
    receivers: Vec<FirmwareJobReceiverStatus>,
    log: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FirmwareJobReceiverStatus {
    ip: String,
    model: String,
    target_file: String,
    status: String,
    phase: String,
    progress: f64,
    current_firmware: Option<String>,
    message: String,
    downgrade_required: bool,
    downgrade_decision: Option<bool>,
    child_job_id: String,
}

#[tauri::command]
fn load_receivers_config() -> Result<Vec<ReceiverSummary>, String> {
    let receivers = read_receivers(GROUPS_JSON_PATH)?;
    Ok(receivers
        .into_iter()
        .map(|rx| ReceiverSummary::from_config(&rx))
        .collect())
}

#[tauri::command]
async fn scan_receivers(state: State<'_, FirmwareJobs>) -> Result<Vec<ReceiverSummary>, String> {
    let single_jobs = Arc::clone(&state.single_jobs);
    tauri::async_runtime::spawn_blocking(move || {
        let receivers = read_receivers(GROUPS_JSON_PATH)?;
        Ok(scan_all(receivers, &single_jobs))
    })
    .await
    .map_err(|err| format!("Receiver scan failed: {err}"))?
}

#[tauri::command]
fn get_update_safety_settings() -> Result<UpdateSafetySettings, String> {
    read_update_safety_settings()
}

#[tauri::command]
fn save_update_safety_settings(
    mut settings: UpdateSafetySettings,
) -> Result<UpdateSafetySettings, String> {
    settings.restart_cycle_minutes = CONVERGENCE_RESTART_CYCLE_MINUTES;
    validate_update_safety_settings(&settings)?;
    write_update_safety_settings(&settings)?;
    Ok(settings)
}

#[tauri::command]
fn open_receiver(ip: String) -> Result<(), String> {
    if ip.parse::<Ipv4Addr>().is_err() {
        return Err(format!("Invalid receiver IP: {ip}"));
    }

    let url = format!("http://{ip}");
    open_url(&url)
}

#[tauri::command]
fn authenticate_operator(username: String, password: String) -> Result<(), String> {
    if username.trim() != APPLICATION_ADMIN_USERNAME {
        return Err("Invalid username or password.".to_string());
    }
    validate_application_password(&password)
        .map_err(|_| "Invalid username or password.".to_string())
}

#[tauri::command]
fn get_receiver_login_details(
    ip: String,
    app_password: String,
) -> Result<ReceiverLoginDetails, String> {
    validate_application_password(&app_password)?;

    if ip.parse::<Ipv4Addr>().is_err() {
        return Err(format!("Invalid receiver IP: {ip}"));
    }

    let receiver = read_receivers(GROUPS_JSON_PATH)?
        .into_iter()
        .find(|rx| rx.addr == ip)
        .ok_or_else(|| format!("Receiver credentials not found for {ip}"))?;

    Ok(ReceiverLoginDetails {
        ip: receiver.addr,
        user: receiver.user,
        password: receiver.pw,
    })
}

#[tauri::command]
fn get_firmware_update_config(mode: String) -> Result<FirmwareConfigResponse, String> {
    firmware_config_response(&mode)
}

#[tauri::command]
async fn list_firmware_releases() -> Result<FirmwareReleaseCatalog, String> {
    tauri::async_runtime::spawn_blocking(fetch_firmware_release_catalog_blocking)
        .await
        .map_err(|err| format!("Firmware release catalog task failed: {err}"))?
}

#[tauri::command]
async fn list_firmware_builds(
    release_id: String,
    release_name: String,
    mode: String,
) -> Result<FirmwareBuildCatalog, String> {
    tauri::async_runtime::spawn_blocking(move || {
        fetch_firmware_build_catalog_blocking(&release_id, &release_name, &mode)
    })
    .await
    .map_err(|err| format!("Firmware build catalog task failed: {err}"))?
}

#[tauri::command]
async fn inspect_firmware_release(
    release_id: String,
    build_id: String,
    mode: String,
    models: Vec<String>,
) -> Result<FirmwareReleaseInspection, String> {
    tauri::async_runtime::spawn_blocking(move || {
        inspect_firmware_release_blocking(&release_id, &build_id, &mode, &models)
    })
    .await
    .map_err(|err| format!("Firmware release inspection task failed: {err}"))?
}

#[tauri::command]
fn add_firmware_config_receiver(
    mode: String,
    ip: String,
    model: String,
) -> Result<FirmwareConfigResponse, String> {
    let ip = ip.trim().to_string();
    if ip.parse::<Ipv4Addr>().is_err() {
        return Err(format!("Invalid receiver IP: {ip}"));
    }

    let model = normalize_model_name(&model);
    if model.is_empty() {
        return Err("Receiver model is required.".to_string());
    }

    let expected_group = firmware_mode_group(&mode)?;
    let receiver = read_receivers(GROUPS_JSON_PATH)?
        .into_iter()
        .find(|receiver| receiver.addr == ip)
        .ok_or_else(|| format!("Receiver {ip} was not found in qa-receivers.json."))?;
    if receiver.group != expected_group {
        return Err(format!(
            "Receiver {ip} belongs to {} in qa-receivers.json and cannot be assigned to {}.",
            firmware_group_name(receiver.group),
            firmware_group_name(expected_group)
        ));
    }

    let mut configs = read_firmware_configs()?;
    for config in configs.values_mut() {
        config.receiver_models.remove(&ip);
    }
    let config = configs
        .get_mut(&mode)
        .ok_or_else(|| format!("Firmware config mode not found: {mode}"))?;
    config.receiver_models.insert(ip, model);
    write_firmware_configs(&configs)?;
    firmware_config_response(&mode)
}

#[tauri::command]
fn remove_firmware_config_receiver(
    mode: String,
    ip: String,
    app_password: String,
) -> Result<FirmwareConfigResponse, String> {
    validate_application_password(&app_password)?;

    let mut configs = read_firmware_configs()?;
    let config = configs
        .get_mut(&mode)
        .ok_or_else(|| format!("Firmware config mode not found: {mode}"))?;
    config.receiver_models.remove(ip.trim());
    write_firmware_configs(&configs)?;
    firmware_config_response(&mode)
}

#[tauri::command]
async fn validate_firmware_plan(
    request: FirmwarePlanRequest,
) -> Result<FirmwarePreflightResult, String> {
    tauri::async_runtime::spawn_blocking(move || validate_firmware_plan_blocking(request))
        .await
        .map_err(|err| format!("Firmware preflight failed: {err}"))?
}

#[tauri::command]
async fn dry_run_single_firmware_update(
    request: FirmwareSingleUpdateRequest,
) -> Result<FirmwareSingleDryRunResult, String> {
    tauri::async_runtime::spawn_blocking(move || dry_run_single_firmware_update_blocking(request))
        .await
        .map_err(|err| format!("Firmware dry run failed: {err}"))?
}

#[tauri::command]
fn pick_local_firmware_package() -> Result<Option<LocalFirmwarePackage>, String> {
    pick_local_firmware_package_blocking()
}

#[tauri::command]
fn pick_local_firmware_packages() -> Result<Vec<LocalFirmwarePackage>, String> {
    pick_local_firmware_packages_blocking()
}

#[tauri::command]
fn inspect_local_firmware_package(path: String) -> Result<LocalFirmwarePackage, String> {
    local_firmware_package_from_path(path.trim())
}

#[tauri::command]
fn start_single_release_firmware_update_job(
    state: State<'_, FirmwareJobs>,
    request: FirmwareSingleUpdateRequest,
) -> Result<FirmwareSingleUpdateJobStatus, String> {
    if request.receiver_ip.trim().is_empty() {
        return Err("Receiver IP is required.".to_string());
    }
    if request.firmware_version.trim().is_empty() {
        return Err("Firmware version is required.".to_string());
    }
    if request.firmware_suffix.trim().is_empty()
        && request
            .package_file
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    {
        return Err("Firmware suffix is required.".to_string());
    }

    let receiver = resolve_single_firmware_receiver(&request, false)?;
    ensure_update_safety_for_receivers(std::slice::from_ref(&receiver))?;
    let job_id = format!("single-fw-{}", now_millis());
    let (reservation, job_limit) = single_or_multiple_reservation(
        &job_id,
        &receiver.credential.addr,
        request.batch_id.as_deref(),
    );
    let lease = state.orchestrator.reserve(reservation, job_limit)?;
    let initial = create_single_update_job_status(
        &job_id,
        &receiver,
        request.batch_id.clone(),
        "Release server firmware update job was queued.",
    );

    {
        let mut jobs = state
            .single_jobs
            .lock()
            .map_err(|_| "Single firmware job store is unavailable.".to_string())?;
        jobs.insert(job_id.clone(), initial.clone());
    }

    let single_jobs = Arc::clone(&state.single_jobs);
    tauri::async_runtime::spawn_blocking(move || {
        run_single_release_firmware_update_job(single_jobs, lease, job_id, request);
    });

    Ok(initial)
}

#[tauri::command]
async fn start_single_local_firmware_update(
    state: State<'_, FirmwareJobs>,
    request: FirmwareSingleLocalUpdateRequest,
) -> Result<FirmwareSingleDryRunResult, String> {
    let single_jobs = Arc::clone(&state.single_jobs);
    let target_url = format!("local://{}", request.local_file_name.trim());
    let initial = start_single_local_firmware_update_job(state, request)?;
    tauri::async_runtime::spawn_blocking(move || loop {
        let job = single_jobs
            .lock()
            .map_err(|_| "Firmware job store is unavailable.")?
            .get(&initial.job_id)
            .cloned()
            .ok_or("Firmware job is unavailable.")?;
        if job.completed_at.is_some() {
            return Ok(FirmwareSingleDryRunResult {
                ip: job.ip,
                model: job.model,
                target_file: job.target_file,
                target_url,
                receiver_name: job.receiver_name,
                firmware_version: job.firmware_version,
                uptime_seconds: job.uptime_seconds,
                status: job.status,
                steps: job.steps,
            });
        }
        thread::sleep(Duration::from_millis(250));
    })
    .await
    .map_err(|err| format!("Single receiver local update failed: {err}"))?
}

#[tauri::command]
fn start_single_local_firmware_update_job(
    state: State<'_, FirmwareJobs>,
    request: FirmwareSingleLocalUpdateRequest,
) -> Result<FirmwareSingleUpdateJobStatus, String> {
    let file_path = Path::new(request.local_file_path.trim());
    if !file_path.is_file() {
        return Err("Local firmware package path is not readable.".to_string());
    }

    validate_local_firmware_file_name(request.local_file_name.trim())?;

    let dry_request = FirmwareSingleUpdateRequest {
        mode: "Auto".to_string(),
        batch_id: request.batch_id.clone(),
        receiver_ip: request.receiver_ip.clone(),
        receiver_model: request.receiver_model.clone(),
        firmware_version: String::new(),
        firmware_suffix: String::new(),
        package_file: None,
        source_mode: Some("local".to_string()),
        local_file_name: Some(request.local_file_name.clone()),
    };
    let receiver = resolve_single_firmware_receiver(&dry_request, true)?;
    ensure_update_safety_for_receivers(std::slice::from_ref(&receiver))?;
    let job_id = format!("single-fw-{}", now_millis());
    let (reservation, job_limit) = single_or_multiple_reservation(
        &job_id,
        &receiver.credential.addr,
        request.batch_id.as_deref(),
    );
    let lease = state.orchestrator.reserve(reservation, job_limit)?;
    let initial = create_single_update_job_status(
        &job_id,
        &receiver,
        request.batch_id.clone(),
        "Local firmware update job was queued.",
    );

    {
        let mut jobs = state
            .single_jobs
            .lock()
            .map_err(|_| "Single firmware job store is unavailable.".to_string())?;
        jobs.insert(job_id.clone(), initial.clone());
    }

    let single_jobs = Arc::clone(&state.single_jobs);
    tauri::async_runtime::spawn_blocking(move || {
        run_single_local_firmware_update_job(single_jobs, lease, job_id, request);
    });

    Ok(initial)
}

fn create_single_update_job_status(
    job_id: &str,
    receiver: &FirmwarePlanReceiver,
    batch_id: Option<String>,
    message: &str,
) -> FirmwareSingleUpdateJobStatus {
    let mut steps = Vec::new();
    push_workflow_step(&mut steps, "Queued", "Running", message);
    let operation_type = match batch_id.as_deref() {
        Some(value) if value.starts_with("group-fw-") => "Group",
        Some(value) if value.starts_with("multiple-fw-") => "Multiple",
        _ => "Single",
    };

    FirmwareSingleUpdateJobStatus {
        job_id: job_id.to_string(),
        batch_id,
        operation_type: Some(operation_type.to_string()),
        group_name: None,
        ip: receiver.credential.addr.clone(),
        model: receiver.model.clone(),
        target_file: receiver.target_file.clone(),
        status: "Running".to_string(),
        phase: "Queued".to_string(),
        message: message.to_string(),
        receiver_name: None,
        firmware_version: None,
        uptime_seconds: None,
        firmware_status: None,
        fw_status: None,
        update_mode: None,
        update_percent: None,
        update_stage: None,
        update_stage_percent: None,
        update_in_progress: false,
        downgrade_required: false,
        downgrade_decision: None,
        active_checksum: None,
        last_seen_at: None,
        clone_file_name: None,
        clone_file_path: None,
        started_at: now_seconds(),
        completed_at: None,
        steps,
    }
}

#[tauri::command]
fn get_single_firmware_update_job_status(
    state: State<'_, FirmwareJobs>,
    job_id: String,
) -> Result<FirmwareSingleUpdateJobStatus, String> {
    let jobs = state
        .single_jobs
        .lock()
        .map_err(|_| "Single firmware job store is unavailable.".to_string())?;
    jobs.get(&job_id)
        .cloned()
        .ok_or_else(|| format!("Single firmware update job not found: {job_id}"))
}

#[tauri::command]
fn cancel_single_firmware_update_job(
    state: State<'_, FirmwareJobs>,
    job_id: String,
) -> Result<FirmwareSingleUpdateJobStatus, String> {
    {
        let jobs = state
            .single_jobs
            .lock()
            .map_err(|_| "Single firmware job store is unavailable.".to_string())?;
        let job = jobs
            .get(&job_id)
            .ok_or_else(|| format!("Single firmware update job not found: {job_id}"))?;
        if job.status == "Completed" || job.status == "Failed" || job.status == "Cancelled" {
            return Ok(job.clone());
        }
    }

    state.orchestrator.request_cancel(&job_id)?;
    update_single_job_message(
        &state.single_jobs,
        &job_id,
        "Cancelling",
        "Running",
        "Running",
        "Cancellation requested. The current receiver operation will finish safely before password recovery.",
    );

    let jobs = state
        .single_jobs
        .lock()
        .map_err(|_| "Single firmware job store is unavailable.".to_string())?;
    jobs.get(&job_id)
        .cloned()
        .ok_or_else(|| format!("Single firmware update job not found: {job_id}"))
}

#[tauri::command]
fn get_single_firmware_update_history() -> Result<Vec<FirmwareSingleUpdateHistoryEntry>, String> {
    let local_history = read_single_update_history().unwrap_or_default();
    let mut synchronization_failed = false;
    for entry in &local_history {
        if let Err(error) = sync_registry_firmware_history_entry(entry) {
            log::warn!("Cannot synchronize local firmware history: {error}");
            if !error.contains("HTTP 409") {
                synchronization_failed = true;
                break;
            }
        }
    }

    match read_registry_firmware_history() {
        Ok(shared_history) => {
            if let Err(error) = write_registry_history_cache(&shared_history) {
                log::warn!("Cannot refresh shared firmware history cache: {error}");
            }
            if synchronization_failed {
                Ok(merge_firmware_history(shared_history, local_history))
            } else {
                Ok(shared_history)
            }
        }
        Err(error) => {
            log::warn!("Shared firmware history is unavailable; using local cache: {error}");
            let cached_history = read_registry_history_cache().unwrap_or_default();
            Ok(merge_firmware_history(cached_history, local_history))
        }
    }
}

#[tauri::command]
fn delete_single_firmware_update_history(
    job_id: String,
    app_password: String,
) -> Result<Vec<FirmwareSingleUpdateHistoryEntry>, String> {
    validate_application_password(&app_password)?;

    let job_id = job_id.trim();
    if job_id.is_empty() {
        return Err("History entry ID is required.".to_string());
    }

    let mut local_history = read_single_update_history()?;
    let local_entry = local_history
        .iter()
        .find(|entry| entry.job_id == job_id)
        .cloned();

    delete_registry_firmware_history(job_id)?;

    if let Some(path) = local_entry
        .as_ref()
        .and_then(|entry| entry.clone_file_path.as_deref())
    {
        delete_clone_file_backup(path)?;
    }

    local_history.retain(|entry| entry.job_id != job_id);
    write_single_update_history(&local_history)?;
    let shared_history = read_registry_firmware_history()?;
    write_registry_history_cache(&shared_history)?;
    Ok(merge_firmware_history(shared_history, local_history))
}

#[tauri::command]
fn download_clone_file_backup(path: String) -> Result<CloneFileDownloadResult, String> {
    if let Some(job_id) = path.trim().strip_prefix("registry://firmware-history/") {
        if job_id.trim().is_empty() {
            return Err("Shared Clone File reference is invalid.".to_string());
        }
        return download_registry_clone_file(job_id.trim());
    }

    let source = PathBuf::from(path.trim());
    if !source.is_file() {
        return Err("Clone file backup does not exist.".to_string());
    }
    let metadata =
        fs::metadata(&source).map_err(|err| format!("Cannot inspect clone file backup: {err}"))?;
    if metadata.len() == 0 {
        return Err(
            "Clone file backup is empty. Run the update again to create a valid backup."
                .to_string(),
        );
    }
    let backup_text = fs::read_to_string(&source)
        .map_err(|err| format!("Cannot read clone file backup: {err}"))?;
    if !is_valid_clone_file_response(&backup_text) {
        return Err(
            "Clone file backup is incomplete. Run the update again to create a valid backup."
                .to_string(),
        );
    }

    let clone_root = qa_receiver_center_data_dir()?.join("clone-files");
    let source_canonical = source
        .canonicalize()
        .map_err(|err| format!("Cannot resolve clone file path: {err}"))?;
    let clone_root_canonical = clone_root
        .canonicalize()
        .map_err(|err| format!("Cannot resolve clone file backup directory: {err}"))?;
    if !source_canonical.starts_with(&clone_root_canonical) {
        return Err("Clone file path is outside QA Receiver Center backups.".to_string());
    }

    let file_name = source_canonical
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Clone file name is invalid.".to_string())?
        .to_string();
    let destination = next_available_download_path(&file_name)?;
    fs::copy(&source_canonical, &destination).map_err(|err| {
        format!(
            "Cannot copy clone file to Downloads {}: {err}",
            destination.display()
        )
    })?;

    Ok(CloneFileDownloadResult {
        file_name,
        destination_path: destination.display().to_string(),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PasswordRecoveryAlert {
    ip: String,
    job_id: String,
    message: String,
    receiver_name: Option<String>,
    model: Option<String>,
}

fn visible_password_recovery_notices(
    notices: Vec<password_recovery_notices::PasswordRecoveryNotice>,
    jobs: &HashMap<String, FirmwareSingleUpdateJobStatus>,
) -> Vec<PasswordRecoveryAlert> {
    notices
        .into_iter()
        .filter(|notice| {
            !jobs
                .get(&notice.job_id)
                .is_some_and(|job| job.completed_at.is_none())
        })
        .map(|notice| {
            let job = jobs.get(&notice.job_id);
            PasswordRecoveryAlert {
                ip: notice.ip,
                job_id: notice.job_id,
                message: job
                    .and_then(password_recovery_error)
                    .unwrap_or(notice.message),
                receiver_name: job.and_then(|job| job.receiver_name.clone()),
                model: job.map(|job| job.model.clone()),
            }
        })
        .collect()
}

#[tauri::command]
fn get_password_recovery_notices(
    state: State<'_, FirmwareJobs>,
) -> Result<Vec<PasswordRecoveryAlert>, String> {
    let notices = password_recovery_notices::read()?;
    let jobs = state
        .single_jobs
        .lock()
        .map_err(|_| "Firmware job store is unavailable.")?;
    Ok(visible_password_recovery_notices(notices, &jobs))
}

#[tauri::command]
fn play_password_recovery_sound(
    state: State<'_, FirmwareJobs>,
    notice_ids: Vec<password_recovery_notices::PasswordRecoveryNoticeId>,
) -> Result<bool, String> {
    let current = get_password_recovery_notices(state)?
        .into_iter()
        .map(
            |notice| password_recovery_notices::PasswordRecoveryNoticeId {
                ip: notice.ip,
                job_id: notice.job_id,
            },
        )
        .collect::<Vec<_>>();
    password_recovery_notices::play_sound_once(&notice_ids, &current)
}

#[tauri::command]
fn restore_receiver_password(
    state: State<'_, FirmwareJobs>,
    receiver_ip: String,
) -> Result<PasswordRestoreResult, String> {
    let ip = receiver_ip.trim();
    if ip.parse::<Ipv4Addr>().is_err() {
        return Err(format!("Invalid receiver IP: {ip}"));
    }

    let _lease = state.orchestrator.reserve(
        FirmwareJobReservation::single(format!("password-recovery-{}", now_millis()), ip),
        MAX_SINGLE_FIRMWARE_JOBS,
    )?;
    let access = receiver_password::operation_lock(ip)?;
    let _access = access
        .try_lock()
        .map_err(|_| "This receiver is busy. Wait before restoring its password.")?;

    let receiver = read_receivers(GROUPS_JSON_PATH)?
        .into_iter()
        .find(|rx| rx.addr == ip)
        .ok_or_else(|| format!("Receiver credentials not found for {ip}"))?;
    let temp_password = firmware_temp_password()?;

    password_recovery_notices::arm(&receiver.addr, &format!("manual-password-{}", now_millis()))?;
    match restore_receiver_password_with_retry(&receiver, &temp_password) {
        Ok(()) => {
            password_recovery_notices::clear(&receiver.addr)?;
            Ok(PasswordRestoreResult {
                ip: receiver.addr,
                status: "Completed".to_string(),
                message: "Receiver password restored from qa-receivers.json.".to_string(),
                restored_at: now_seconds(),
            })
        }
        Err(change_error) => Err(format!(
            "Password restore failed for {}: {}",
            receiver.addr, change_error
        )),
    }
}

#[tauri::command]
async fn force_restore_firmware_job_passwords(
    state: State<'_, FirmwareJobs>,
    job_id: String,
) -> Result<FirmwarePasswordRecoveryBatchResult, String> {
    let receiver_jobs = {
        let jobs = state
            .jobs
            .lock()
            .map_err(|_| "Firmware job store is unavailable.".to_string())?;
        let job = jobs
            .get(&job_id)
            .ok_or_else(|| format!("Firmware job not found: {job_id}"))?;
        job.receivers
            .iter()
            .map(|receiver| (receiver.ip.clone(), receiver.child_job_id.clone()))
            .collect::<Vec<_>>()
    };
    let recovery_lease = state.orchestrator.reserve(
        FirmwareJobReservation {
            job_id: format!("password-recovery-{}", now_millis()),
            kind: FirmwareJobKind::Multiple,
            receiver_ips: receiver_jobs.iter().map(|(ip, _)| ip.clone()).collect(),
            max_parallel_receivers: MAX_PASSWORD_RECOVERY_PARALLEL_RECEIVERS,
        },
        1,
    )?;
    let receiver_phases = state
        .single_jobs
        .lock()
        .map_err(|_| "Single firmware job store is unavailable.".to_string())?
        .iter()
        .map(|(job_id, job)| (job_id.clone(), job.phase.clone()))
        .collect::<HashMap<_, _>>();
    let receivers = receiver_jobs
        .into_iter()
        .map(|(ip, child_job_id)| {
            let phase = receiver_phases
                .get(&child_job_id)
                .cloned()
                .unwrap_or_default();
            (ip, phase)
        })
        .collect();
    let group_jobs = Arc::clone(&state.jobs);
    let worker_job_id = job_id.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let _lease = recovery_lease;
        force_restore_firmware_job_passwords_blocking(group_jobs, worker_job_id, receivers)
    })
    .await
    .map_err(|err| format!("Password recovery task stopped unexpectedly: {err}"))?
}

fn force_restore_firmware_job_passwords_blocking(
    group_jobs: Arc<Mutex<HashMap<String, FirmwareJobStatus>>>,
    job_id: String,
    receivers_to_restore: Vec<(String, String)>,
) -> Result<FirmwarePasswordRecoveryBatchResult, String> {
    let configured_receivers: HashMap<_, _> = read_receivers(GROUPS_JSON_PATH)?
        .into_iter()
        .map(|receiver| (receiver.addr.clone(), receiver))
        .collect();
    let temporary_password = firmware_temp_password()?;
    let mut workers = Vec::with_capacity(receivers_to_restore.len());

    for (ip, phase) in receivers_to_restore {
        let Some(receiver) = configured_receivers.get(&ip).cloned() else {
            workers.push((ip.clone(), None));
            continue;
        };
        let password = temporary_password.clone();
        workers.push((
            ip,
            Some(thread::spawn(move || {
                force_restore_receiver_password(&receiver, &password, &phase)
            })),
        ));
    }

    let mut receivers = Vec::with_capacity(workers.len());
    for (ip, worker) in workers {
        let result = match worker {
            Some(worker) => worker.join().unwrap_or_else(|_| PasswordRestoreResult {
                ip,
                status: "Failed".to_string(),
                message: "Password recovery worker stopped unexpectedly.".to_string(),
                restored_at: now_seconds(),
            }),
            None => PasswordRestoreResult {
                ip,
                status: "Failed".to_string(),
                message: "Receiver credentials were not found in qa-receivers.json.".to_string(),
                restored_at: now_seconds(),
            },
        };
        receivers.push(result);
    }

    let restored_count = receivers
        .iter()
        .filter(|receiver| receiver.status == "Completed")
        .count();
    let already_restored_count = receivers
        .iter()
        .filter(|receiver| receiver.status == "AlreadyRestored")
        .count();
    let deferred_count = receivers
        .iter()
        .filter(|receiver| receiver.status == "Deferred")
        .count();
    let failed_count = receivers
        .iter()
        .filter(|receiver| receiver.status == "Failed")
        .count();
    let status = if failed_count > 0 {
        "PartialFailure"
    } else if deferred_count > 0 {
        "Deferred"
    } else {
        "Completed"
    }
    .to_string();

    if let Ok(mut jobs) = group_jobs.lock() {
        if let Some(job) = jobs.get_mut(&job_id) {
            job.log.push(format!(
                "[{}] Forced password recovery: {} restored, {} already configured, {} deferred, {} failed.",
                now_seconds(),
                restored_count,
                already_restored_count,
                deferred_count,
                failed_count
            ));
        }
    }

    Ok(FirmwarePasswordRecoveryBatchResult {
        job_id,
        status,
        total_count: receivers.len(),
        restored_count,
        already_restored_count,
        deferred_count,
        failed_count,
        receivers,
    })
}

fn force_restore_receiver_password(
    receiver: &ReceiverCredential,
    temporary_password: &str,
    phase: &str,
) -> PasswordRestoreResult {
    let access = match receiver_password::operation_lock(&receiver.addr) {
        Ok(access) => access,
        Err(error) => {
            return PasswordRestoreResult {
                ip: receiver.addr.clone(),
                status: "Failed".into(),
                message: error,
                restored_at: now_seconds(),
            }
        }
    };
    let Ok(_access) = access.try_lock() else {
        return PasswordRestoreResult {
            ip: receiver.addr.clone(),
            status: "Deferred".into(),
            message: "Receiver is busy. Password recovery was deferred.".into(),
            restored_at: now_seconds(),
        };
    };
    if phase == "ChangePassword" || phase == "UploadFirmware" {
        return PasswordRestoreResult {
            ip: receiver.addr.clone(),
            status: "Deferred".to_string(),
            message: format!(
                "Password recovery was deferred because the receiver is in the critical {} phase.",
                format_workflow_state(phase)
            ),
            restored_at: now_seconds(),
        };
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(6))
        .connect_timeout(Duration::from_secs(3))
        .build();
    if let Ok(client) = client {
        if receiver_firmware_live_probe(&client, receiver, temporary_password)
            .is_ok_and(|probe| probe.update_in_progress)
        {
            return PasswordRestoreResult {
                ip: receiver.addr.clone(),
                status: "Deferred".to_string(),
                message: "Receiver is still programming firmware. Password recovery was deferred for safety."
                    .to_string(),
                restored_at: now_seconds(),
            };
        }
    }

    match password_recovery_notices::arm(
        &receiver.addr,
        &format!("manual-password-{}", now_millis()),
    )
    .and_then(|()| restore_receiver_password_with_retry(receiver, temporary_password))
    .and_then(|()| password_recovery_notices::clear(&receiver.addr))
    {
        Ok(()) => PasswordRestoreResult {
            ip: receiver.addr.clone(),
            status: "Completed".to_string(),
            message: "Receiver password was restored from qa-receivers.json.".to_string(),
            restored_at: now_seconds(),
        },
        Err(error) => PasswordRestoreResult {
            ip: receiver.addr.clone(),
            status: "Failed".to_string(),
            message: error,
            restored_at: now_seconds(),
        },
    }
}

#[tauri::command]
fn start_firmware_update_job(
    state: State<'_, FirmwareJobs>,
    request: FirmwarePlanRequest,
) -> Result<FirmwareJobStatus, String> {
    let uses_local_packages = request
        .local_package_paths
        .as_ref()
        .is_some_and(|paths| !paths.is_empty());
    if request.firmware_version.trim().is_empty() {
        return Err("Firmware version or release is required.".to_string());
    }
    if !uses_local_packages
        && request.package_overrides.is_none()
        && request.firmware_suffix.trim().is_empty()
    {
        return Err("Firmware suffix or package name is required.".to_string());
    }

    let (_mode_config, group, receivers) = resolve_firmware_receivers(&request)?;
    if receivers.is_empty() {
        return Err("Select at least one receiver before starting a firmware job.".to_string());
    }
    ensure_update_safety_for_receivers(&receivers)?;

    let job_id = format!("group-fw-{}", now_millis());
    let max_parallel_receivers = receiver_concurrency(request.parallel, receivers.len());
    let lease = state.orchestrator.reserve(
        FirmwareJobReservation {
            job_id: job_id.clone(),
            kind: FirmwareJobKind::Group,
            receiver_ips: receivers
                .iter()
                .map(|receiver| receiver.credential.addr.clone())
                .collect(),
            max_parallel_receivers,
        },
        MAX_GROUP_FIRMWARE_JOBS,
    )?;
    let receiver_jobs: Vec<_> = receivers
        .into_iter()
        .enumerate()
        .map(|(index, receiver)| {
            let child_job_id = format!("{job_id}-rx-{:03}", index + 1);
            (receiver, child_job_id)
        })
        .collect();
    let log = vec![
        format!(
            "[{}] Group update started for {} receiver(s).",
            now_seconds(),
            receiver_jobs.len()
        ),
        format!(
            "[{}] Receiver concurrency is limited to {}. Password recovery is armed for every receiver.",
            now_seconds(),
            max_parallel_receivers
        ),
    ];

    let status = FirmwareJobStatus {
        job_id: job_id.clone(),
        mode: request.mode.clone(),
        group,
        group_name: firmware_group_name(group).to_string(),
        firmware_version: request.firmware_version.trim().to_string(),
        firmware_suffix: if uses_local_packages {
            "Local packages".to_string()
        } else if request.package_overrides.is_some() {
            "Release catalog".to_string()
        } else {
            request.firmware_suffix.trim().to_string()
        },
        parallel: request.parallel,
        status: "Running".to_string(),
        progress: 0.0,
        created_at: now_seconds(),
        completed_at: None,
        receivers: receiver_jobs
            .iter()
            .map(|(receiver, child_job_id)| FirmwareJobReceiverStatus {
                ip: receiver.credential.addr.clone(),
                model: receiver.model.clone(),
                target_file: receiver.target_file.clone(),
                status: "Running".to_string(),
                phase: "Queued".to_string(),
                progress: 0.0,
                current_firmware: None,
                message: "Waiting for the group update worker.".to_string(),
                downgrade_required: false,
                downgrade_decision: None,
                child_job_id: child_job_id.clone(),
            })
            .collect(),
        log,
    };

    {
        let mut single_jobs = state
            .single_jobs
            .lock()
            .map_err(|_| "Single firmware job store is unavailable.".to_string())?;
        for (receiver, child_job_id) in &receiver_jobs {
            let mut child_status = create_single_update_job_status(
                child_job_id,
                receiver,
                Some(job_id.clone()),
                "Waiting for the group update worker.",
            );
            child_status.operation_type = Some("Group".to_string());
            child_status.group_name = Some(request.mode.clone());
            single_jobs.insert(child_job_id.clone(), child_status);
        }
    }
    {
        let mut jobs = state
            .jobs
            .lock()
            .map_err(|_| "Firmware job store is unavailable.".to_string())?;
        jobs.insert(job_id.clone(), status.clone());
    }

    let group_jobs = Arc::clone(&state.jobs);
    let single_jobs = Arc::clone(&state.single_jobs);
    let spawned_job_id = job_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        run_group_release_firmware_update_job(
            group_jobs,
            single_jobs,
            lease,
            spawned_job_id,
            receiver_jobs,
        );
    });

    Ok(status)
}

#[tauri::command]
fn get_firmware_job_status(
    state: State<'_, FirmwareJobs>,
    job_id: String,
) -> Result<FirmwareJobStatus, String> {
    sync_group_job_from_children(&state.jobs, &state.single_jobs, &job_id);
    let jobs = state
        .jobs
        .lock()
        .map_err(|_| "Firmware job store is unavailable.".to_string())?;
    jobs.get(&job_id)
        .cloned()
        .ok_or_else(|| format!("Firmware job not found: {job_id}"))
}

#[tauri::command]
fn stop_firmware_update_job(
    state: State<'_, FirmwareJobs>,
    job_id: String,
) -> Result<FirmwareJobStatus, String> {
    {
        let jobs = state
            .jobs
            .lock()
            .map_err(|_| "Firmware job store is unavailable.".to_string())?;
        let job = jobs
            .get(&job_id)
            .ok_or_else(|| format!("Firmware job not found: {job_id}"))?;
        if job.status == "Completed" || job.status == "Failed" || job.status == "Cancelled" {
            return Ok(job.clone());
        }
    }

    state.orchestrator.request_cancel(&job_id)?;
    let mut jobs = state
        .jobs
        .lock()
        .map_err(|_| "Firmware job store is unavailable.".to_string())?;
    let job = jobs
        .get_mut(&job_id)
        .ok_or_else(|| format!("Firmware job not found: {job_id}"))?;
    job.status = "Cancelling".to_string();
    job.log.push(format!(
        "[{}] Cancellation requested. Queued work will stop immediately; active receivers will stop as soon as firmware is idle, then restore configured passwords.",
        now_seconds()
    ));
    Ok(job.clone())
}

#[tauri::command]
fn resolve_firmware_downgrade(
    state: State<'_, FirmwareJobs>,
    job_id: String,
    approved: bool,
) -> Result<FirmwareSingleUpdateJobStatus, String> {
    let mut jobs = state
        .single_jobs
        .lock()
        .map_err(|_| "Single firmware job store is unavailable.".to_string())?;
    let job = jobs
        .get_mut(&job_id)
        .ok_or_else(|| format!("Firmware receiver job not found: {job_id}"))?;
    if !job.downgrade_required || job.phase != "DowngradeApproval" {
        return Err("This receiver is not waiting for downgrade approval.".to_string());
    }

    job.downgrade_decision = Some(approved);
    job.message = if approved {
        "Downgrade was approved by the operator. Continuing the protected update workflow."
            .to_string()
    } else {
        "Downgrade was cancelled by the operator. No receiver password was changed.".to_string()
    };
    Ok(job.clone())
}

fn set_single_job_downgrade_gate(
    single_jobs: &Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    job_id: &str,
    message: &str,
) {
    if let Ok(mut jobs) = single_jobs.lock() {
        if let Some(job) = jobs.get_mut(job_id) {
            job.downgrade_required = true;
            job.downgrade_decision = None;
            job.phase = "DowngradeApproval".to_string();
            job.status = "Running".to_string();
            job.message = message.to_string();
            push_workflow_step(&mut job.steps, "DowngradeApproval", "Waiting", message);
        }
    }
}

fn wait_for_downgrade_decision(
    single_jobs: &Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    control: &FirmwareJobControl,
    job_id: &str,
) -> Result<bool, String> {
    let started_at = Instant::now();
    let timeout = Duration::from_secs(30 * 60);

    while started_at.elapsed() < timeout {
        if control.is_cancel_requested() {
            return Ok(false);
        }

        let decision = single_jobs
            .lock()
            .map_err(|_| "Single firmware job store is unavailable.".to_string())?
            .get(job_id)
            .and_then(|job| job.downgrade_decision);
        if let Some(approved) = decision {
            return Ok(approved);
        }

        thread::sleep(Duration::from_millis(250));
    }

    Err("Timed out waiting for operator downgrade approval.".to_string())
}

fn run_group_release_firmware_update_job(
    group_jobs: Arc<Mutex<HashMap<String, FirmwareJobStatus>>>,
    single_jobs: Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    lease: FirmwareJobLease,
    group_job_id: String,
    receiver_jobs: Vec<(FirmwarePlanReceiver, String)>,
) {
    let control = lease.control();
    let result = catch_unwind(AssertUnwindSafe(|| {
        run_group_release_firmware_update_job_inner(
            &group_jobs,
            &single_jobs,
            &lease,
            &control,
            &group_job_id,
            &receiver_jobs,
        )
    }));

    if !matches!(result, Ok(Ok(()))) {
        let cancelled = control.is_cancel_requested();
        let message = match result {
            Ok(Err(err)) => err,
            Err(_) => {
                "Group firmware update stopped unexpectedly. Password recovery was attempted."
                    .to_string()
            }
            Ok(Ok(())) => String::new(),
        };
        for (_, child_job_id) in &receiver_jobs {
            if !single_job_is_terminal(&single_jobs, child_job_id) {
                update_single_job_step(
                    &single_jobs,
                    child_job_id,
                    if cancelled { "Cancelled" } else { "Failed" },
                    if cancelled { "Cancelled" } else { "Failed" },
                    if cancelled { "Cancelled" } else { "Failed" },
                    &message,
                );
                complete_single_job(
                    &single_jobs,
                    child_job_id,
                    if cancelled { "Cancelled" } else { "Failed" },
                    &message,
                );
            }
        }
    }

    sync_group_job_from_children(&group_jobs, &single_jobs, &group_job_id);
}

fn run_group_release_firmware_update_job_inner(
    group_jobs: &Arc<Mutex<HashMap<String, FirmwareJobStatus>>>,
    single_jobs: &Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    lease: &FirmwareJobLease,
    control: &FirmwareJobControl,
    group_job_id: &str,
    receiver_jobs: &[(FirmwarePlanReceiver, String)],
) -> Result<(), String> {
    ensure_firmware_job_not_cancelled(control)?;
    let mut package_paths = HashMap::<String, Result<PathBuf, String>>::new();
    let mut unique_packages = BTreeMap::<String, (String, Option<PathBuf>)>::new();
    for (receiver, _) in receiver_jobs {
        unique_packages
            .entry(receiver.target_file.clone())
            .or_insert((
                receiver.target_url.clone(),
                receiver.local_package_path.clone(),
            ));
    }

    for (target_file, (target_url, local_package_path)) in unique_packages {
        ensure_firmware_job_not_cancelled(control)?;
        for (receiver, child_job_id) in receiver_jobs {
            if receiver.target_file == target_file {
                update_single_job_step(
                    single_jobs,
                    child_job_id,
                    "DownloadPackage",
                    "Running",
                    "Running",
                    if local_package_path.is_some() {
                        "Validating the selected local model package."
                    } else {
                        "Downloading the shared model package from the release server."
                    },
                );
            }
        }

        let download = if let Some(path) = local_package_path {
            if path.is_file() {
                Ok(path)
            } else {
                Err(format!(
                    "Selected local firmware package is no longer readable: {}",
                    path.display()
                ))
            }
        } else {
            download_release_firmware_package(&target_url, &target_file, "group", group_job_id)
        };
        match &download {
            Ok(_) => {
                for (receiver, child_job_id) in receiver_jobs {
                    if receiver.target_file == target_file {
                        update_single_job_step(
                            single_jobs,
                            child_job_id,
                            "DownloadPackage",
                            "Completed",
                            "Running",
                            "Firmware package is ready for receiver upload.",
                        );
                    }
                }
            }
            Err(err) => {
                for (receiver, child_job_id) in receiver_jobs {
                    if receiver.target_file == target_file {
                        update_single_job_step(
                            single_jobs,
                            child_job_id,
                            "DownloadPackage",
                            "Failed",
                            "Failed",
                            err,
                        );
                        complete_single_job(single_jobs, child_job_id, "Failed", err);
                    }
                }
            }
        }
        package_paths.insert(target_file, download);
        sync_group_job_from_children(group_jobs, single_jobs, group_job_id);
    }

    let runnable: Vec<_> = receiver_jobs
        .iter()
        .filter_map(|(receiver, child_job_id)| {
            package_paths
                .get(&receiver.target_file)
                .and_then(|result| result.as_ref().ok())
                .map(|path| (receiver.clone(), child_job_id.clone(), path.clone()))
        })
        .collect();

    for batch in runnable.chunks(lease.max_parallel_receivers().max(1)) {
        if control.is_cancel_requested() {
            break;
        }
        let handles: Vec<_> = batch
            .iter()
            .cloned()
            .map(|(receiver, child_job_id, package_path)| {
                let worker_jobs = Arc::clone(single_jobs);
                let worker_control = control.clone();
                let worker_group_job_id = group_job_id.to_string();
                thread::spawn(move || {
                    run_group_receiver_worker(
                        worker_jobs,
                        worker_control,
                        worker_group_job_id,
                        child_job_id,
                        receiver,
                        package_path,
                    );
                })
            })
            .collect();
        for handle in handles {
            let _ = handle.join();
        }
        sync_group_job_from_children(group_jobs, single_jobs, group_job_id);
    }

    if control.is_cancel_requested() {
        let message = "Group firmware update was cancelled before this receiver started.";
        for (_, child_job_id) in receiver_jobs {
            if !single_job_is_terminal(single_jobs, child_job_id) {
                update_single_job_step(
                    single_jobs,
                    child_job_id,
                    "Cancelled",
                    "Cancelled",
                    "Cancelled",
                    message,
                );
                complete_single_job(single_jobs, child_job_id, "Cancelled", message);
            }
        }
    }

    sync_group_job_from_children(group_jobs, single_jobs, group_job_id);
    Ok(())
}

fn run_group_receiver_worker(
    single_jobs: Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    control: FirmwareJobControl,
    group_job_id: String,
    child_job_id: String,
    receiver: FirmwarePlanReceiver,
    package_path: PathBuf,
) {
    let local_request = FirmwareSingleLocalUpdateRequest {
        batch_id: Some(group_job_id),
        receiver_ip: receiver.credential.addr,
        receiver_model: Some(receiver.model),
        local_file_name: receiver.target_file,
        local_file_path: package_path.display().to_string(),
    };
    match catch_unwind(AssertUnwindSafe(|| {
        run_single_local_firmware_update_job_inner(
            &single_jobs,
            &control,
            &child_job_id,
            local_request,
        )
    })) {
        Ok(Ok(())) => {}
        Ok(Err(err)) => {
            complete_worker_error(&single_jobs, &control, &child_job_id, &err);
        }
        Err(_) => {
            let message = "Receiver update stopped unexpectedly. Password recovery was attempted.";
            update_single_job_step(
                &single_jobs,
                &child_job_id,
                "Failed",
                "Failed",
                "Failed",
                message,
            );
            complete_single_job(&single_jobs, &child_job_id, "Failed", message);
        }
    }
}

fn single_job_is_terminal(
    single_jobs: &Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    job_id: &str,
) -> bool {
    single_jobs
        .lock()
        .ok()
        .and_then(|jobs| jobs.get(job_id).map(|job| job.status.clone()))
        .is_some_and(|status| status == "Completed" || status == "Failed" || status == "Cancelled")
}

fn sync_group_job_from_children(
    group_jobs: &Arc<Mutex<HashMap<String, FirmwareJobStatus>>>,
    single_jobs: &Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    group_job_id: &str,
) {
    let child_snapshot = match single_jobs.lock() {
        Ok(jobs) => jobs.clone(),
        Err(_) => return,
    };
    let mut jobs = match group_jobs.lock() {
        Ok(jobs) => jobs,
        Err(_) => return,
    };
    let Some(group_job) = jobs.get_mut(group_job_id) else {
        return;
    };

    for receiver in &mut group_job.receivers {
        let Some(child) = child_snapshot.get(&receiver.child_job_id) else {
            continue;
        };
        receiver.status = child.status.clone();
        receiver.phase = child.phase.clone();
        receiver.progress = if child.status == "Completed"
            || child.status == "Failed"
            || child.status == "Cancelled"
        {
            100.0
        } else {
            child.update_percent.unwrap_or(0.0).clamp(0.0, 99.0)
        };
        receiver.current_firmware = child.firmware_version.clone();
        receiver.message = child.message.clone();
        receiver.downgrade_required = child.downgrade_required;
        receiver.downgrade_decision = child.downgrade_decision;
    }

    let receiver_count = group_job.receivers.len().max(1);
    group_job.progress = group_job
        .receivers
        .iter()
        .map(|receiver| receiver.progress)
        .sum::<f64>()
        / receiver_count as f64;
    let all_terminal = group_job.receivers.iter().all(|receiver| {
        receiver.status == "Completed"
            || receiver.status == "Failed"
            || receiver.status == "Cancelled"
    });
    if all_terminal {
        let failed = group_job
            .receivers
            .iter()
            .filter(|receiver| receiver.status == "Failed")
            .count();
        let cancelled = group_job
            .receivers
            .iter()
            .filter(|receiver| receiver.status == "Cancelled")
            .count();
        group_job.status = if failed > 0 {
            "Failed".to_string()
        } else if cancelled > 0 {
            "Cancelled".to_string()
        } else {
            "Completed".to_string()
        };
        group_job.progress = 100.0;
        if group_job.completed_at.is_none() {
            group_job.completed_at = Some(now_seconds());
            group_job.log.push(format!(
                "[{}] Group update finished: {} completed, {} cancelled, {} failed.",
                now_seconds(),
                group_job.receivers.len().saturating_sub(failed + cancelled),
                cancelled,
                failed
            ));
        }
    } else if group_job.status != "Cancelling" {
        group_job.status = "Running".to_string();
    }
}

impl ReceiverSummary {
    fn from_config(rx: &ReceiverCredential) -> Self {
        Self {
            ip: rx.addr.clone(),
            group: rx.group,
            status: "pending".to_string(),
            receiver_name: None,
            firmware_version: None,
            firmware_date: None,
            rtk_version: None,
            test_mode: None,
            uptime_seconds: None,
            celsius: None,
            sv_solved: None,
            fix_type: None,
            error_warning: None,
            needs_review: false,
            last_error: None,
        }
    }

    fn offline(rx: &ReceiverCredential, message: String) -> Self {
        Self {
            status: "offline".to_string(),
            last_error: Some(message),
            ..Self::from_config(rx)
        }
    }
}

fn firmware_group_name(group: u8) -> &'static str {
    match group {
        1 => "[QA] Beta",
        2 => "[QA] Alpha / Top of tree",
        3 => "[QA] Group 3",
        4 => "[QA] Group 4",
        _ => "Unknown",
    }
}

fn firmware_mode_group(mode: &str) -> Result<u8, String> {
    match mode {
        "Beta" => Ok(1),
        "Alpha" => Ok(2),
        _ => Err(format!("Unsupported firmware mode: {mode}")),
    }
}

fn firmware_config_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("QA_RECEIVER_FIRMWARE_CONFIG")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
    {
        return Ok(path);
    }

    let path = qa_receiver_center_data_dir()?.join("firmware-configs.json");
    if !path.is_file() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("Cannot create application data directory: {err}"))?;
        }
        fs::write(&path, DEFAULT_FIRMWARE_CONFIG).map_err(|err| {
            format!(
                "Cannot create default firmware configuration at {}: {err}",
                path.display()
            )
        })?;
    }
    Ok(path)
}

fn read_firmware_configs() -> Result<BTreeMap<String, FirmwareConfigMode>, String> {
    let path = firmware_config_path()?;
    let raw = fs::read_to_string(&path).map_err(|err| {
        format!(
            "Cannot read firmware configuration at {}: {err}",
            path.display()
        )
    })?;
    serde_json::from_str(&raw).map_err(|err| {
        format!(
            "Invalid firmware configuration at {}: {err}",
            path.display()
        )
    })
}

fn write_firmware_configs(configs: &BTreeMap<String, FirmwareConfigMode>) -> Result<(), String> {
    let path = firmware_config_path()?;
    let raw = serde_json::to_string_pretty(configs)
        .map_err(|err| format!("Cannot serialize firmware configs.json: {err}"))?;
    fs::write(&path, raw).map_err(|err| {
        format!(
            "Cannot write firmware configuration at {}: {err}",
            path.display()
        )
    })
}

fn firmware_config_response(mode: &str) -> Result<FirmwareConfigResponse, String> {
    let configs = read_firmware_configs()?;
    let config = configs
        .get(mode)
        .ok_or_else(|| format!("Firmware config mode not found: {mode}"))?;
    let group = firmware_mode_group(mode)?;
    let credentials = read_receivers(GROUPS_JSON_PATH)?;

    let receivers = firmware_receivers_for_group(&configs, mode, group, credentials);

    Ok(FirmwareConfigResponse {
        mode: mode.to_string(),
        group,
        group_name: firmware_group_name(group).to_string(),
        firmware_version: config.firmware_version.clone(),
        default_firmware_suffix: config
            .firmware_suffix
            .get("default")
            .cloned()
            .unwrap_or_default(),
        logo: config.logo.clone(),
        receivers,
    })
}

fn firmware_receivers_for_group(
    configs: &BTreeMap<String, FirmwareConfigMode>,
    mode: &str,
    group: u8,
    credentials: Vec<ReceiverCredential>,
) -> Vec<FirmwareReceiver> {
    let mut receivers: Vec<_> = credentials
        .into_iter()
        .filter(|receiver| receiver.group == group)
        .map(|receiver| FirmwareReceiver {
            model: firmware_model_for_receiver(configs, mode, &receiver.addr)
                .unwrap_or_else(|| "unknown".to_string()),
            ip: receiver.addr,
            group,
            group_name: firmware_group_name(group).to_string(),
            has_credentials: true,
        })
        .collect();
    receivers.sort_by_key(|receiver| ip_sort_key(&receiver.ip));
    receivers
}

fn firmware_model_for_receiver(
    configs: &BTreeMap<String, FirmwareConfigMode>,
    preferred_mode: &str,
    ip: &str,
) -> Option<String> {
    configs
        .get(preferred_mode)
        .and_then(|config| config.receiver_models.get(ip))
        .or_else(|| {
            configs
                .values()
                .find_map(|config| config.receiver_models.get(ip))
        })
        .map(|model| normalize_model_name(model))
        .filter(|model| !model.is_empty())
}

fn normalize_model_name(model: &str) -> String {
    model.trim().to_ascii_lowercase()
}

fn required_firmware_package_extension(model: &str) -> Option<&'static str> {
    match normalize_model_name(model).as_str() {
        "silk" | "stella" | "yeager" => Some("img"),
        _ => None,
    }
}

fn firmware_suffix_with_required_extension(model: &str, suffix: &str) -> String {
    let Some(required_extension) = required_firmware_package_extension(model) else {
        return suffix.to_string();
    };
    let lower = suffix.to_ascii_lowercase();
    for current_extension in [".timg", ".img"] {
        if lower.ends_with(current_extension) {
            return format!(
                "{}.{}",
                &suffix[..suffix.len() - current_extension.len()],
                required_extension
            );
        }
    }
    suffix.to_string()
}

fn firmware_suffix_for_model(
    config: &FirmwareConfigMode,
    model: &str,
    default_suffix_override: &str,
) -> String {
    let suffix = if let Some(model_suffix) = config.firmware_suffix.get(model) {
        model_suffix.clone()
    } else {
        let override_suffix = default_suffix_override.trim();
        if !override_suffix.is_empty() {
            override_suffix.to_string()
        } else {
            config
                .firmware_suffix
                .get("default")
                .cloned()
                .unwrap_or_default()
        }
    };
    firmware_suffix_with_required_extension(model, &suffix)
}

fn firmware_target_for_model(
    config: &FirmwareConfigMode,
    firmware_version: &str,
    default_suffix_override: &str,
    model: &str,
) -> (String, String) {
    let suffix = firmware_suffix_for_model(config, model, default_suffix_override);
    let filename = format!("{model}{suffix}");
    let url = format!(
        "{}/{}/{}",
        FIRMWARE_RELEASE_BASE_URL,
        firmware_version.trim(),
        filename
    );
    (filename, url)
}

fn firmware_target_for_exact_package(
    firmware_version: &str,
    package_file: &str,
) -> Result<(String, String), String> {
    let release_id = validate_firmware_release_id(firmware_version)?;
    validate_firmware_package_file_name(package_file)?;
    let release_url = firmware_release_url(&release_id)?;
    let package_url = release_url
        .join(package_file)
        .map_err(|err| format!("Invalid firmware package URL for {package_file}: {err}"))?;
    Ok((package_file.to_string(), package_url.to_string()))
}

fn firmware_catalog_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(6))
        .user_agent("QA-Receiver-Center/0.1.0")
        .build()
        .map_err(|err| format!("Cannot create firmware catalog HTTP client: {err}"))
}

fn firmware_release_base_url() -> Result<Url, String> {
    Url::parse(&format!(
        "{}/",
        FIRMWARE_RELEASE_BASE_URL.trim_end_matches('/')
    ))
    .map_err(|err| format!("Invalid firmware release server URL: {err}"))
}

fn firmware_release_url(release_id: &str) -> Result<Url, String> {
    let release_id = validate_firmware_release_id(release_id)?;
    firmware_release_base_url()?
        .join(&format!("{release_id}/"))
        .map_err(|err| format!("Invalid firmware release URL: {err}"))
}

fn firmware_release_index_url(release_id: &str) -> Result<Url, String> {
    firmware_release_url(release_id)?
        .join("index.html")
        .map_err(|err| format!("Invalid firmware release index URL: {err}"))
}

fn firmware_build_url(release_id: &str, build_id: &str) -> Result<Url, String> {
    let build_id = validate_firmware_build_id(build_id)?;
    firmware_release_url(release_id)?
        .join(&format!("{build_id}.html"))
        .map_err(|err| format!("Invalid firmware build URL: {err}"))
}

fn validate_firmware_release_id(release_id: &str) -> Result<String, String> {
    let release_id = release_id.trim();
    if release_id.is_empty() {
        return Err("Select a firmware release from the server catalog.".to_string());
    }

    if release_id.contains('/')
        || release_id.contains('\\')
        || release_id.contains("..")
        || release_id.contains('?')
        || release_id.contains('#')
        || !release_id
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || b"._-+%".contains(&value))
    {
        return Err("Invalid firmware release identifier.".to_string());
    }

    let decoded = percent_decode_catalog_segment(release_id);
    if decoded.contains('/') || decoded.contains('\\') || decoded.contains("..") {
        return Err("Invalid firmware release identifier.".to_string());
    }

    Ok(release_id.to_string())
}

fn validate_firmware_build_id(build_id: &str) -> Result<String, String> {
    let build_id = build_id.trim();
    if build_id.is_empty() {
        return Err("Select a firmware build from the release catalog.".to_string());
    }

    if build_id.contains('/')
        || build_id.contains('\\')
        || build_id.contains("..")
        || build_id.contains('?')
        || build_id.contains('#')
        || !build_id
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || b"._-+%".contains(&value))
    {
        return Err("Invalid firmware build identifier.".to_string());
    }

    let decoded = percent_decode_catalog_segment(build_id);
    if decoded.contains('/') || decoded.contains('\\') || decoded.contains("..") {
        return Err("Invalid firmware build identifier.".to_string());
    }

    Ok(build_id.to_string())
}

fn validate_firmware_package_file_name(file_name: &str) -> Result<(), String> {
    let file_name = file_name.trim();
    if file_name.is_empty()
        || Path::new(file_name)
            .file_name()
            .and_then(|value| value.to_str())
            != Some(file_name)
        || file_name.contains('/')
        || file_name.contains('\\')
    {
        return Err("Invalid firmware package file name.".to_string());
    }
    validate_local_firmware_file_name(file_name)
}

fn fetch_firmware_release_catalog_blocking() -> Result<FirmwareReleaseCatalog, String> {
    let client = firmware_catalog_client()?;
    let base_url = firmware_release_base_url()?;
    let html = client
        .get(base_url.clone())
        .send()
        .map_err(|err| format!("Cannot reach firmware release server: {err}"))?
        .error_for_status()
        .map_err(|err| format!("Firmware release server returned an error: {err}"))?
        .text()
        .map_err(|err| format!("Cannot read firmware release catalog: {err}"))?;
    let releases = parse_firmware_release_links(&base_url, &html)?;

    if releases.is_empty() {
        return Err("The firmware release server returned no release pages.".to_string());
    }

    Ok(FirmwareReleaseCatalog {
        source_url: base_url.to_string(),
        refreshed_at: now_seconds(),
        releases,
    })
}

fn fetch_firmware_build_catalog_blocking(
    release_id: &str,
    release_name: &str,
    mode: &str,
) -> Result<FirmwareBuildCatalog, String> {
    let release_id = validate_firmware_release_id(release_id)?;
    firmware_mode_group(mode)?;
    let index_url = firmware_release_index_url(&release_id)?;
    let client = firmware_catalog_client()?;
    let html = client
        .get(index_url.clone())
        .send()
        .map_err(|err| format!("Cannot reach firmware release {release_id}: {err}"))?
        .error_for_status()
        .map_err(|err| format!("Firmware release {release_id} returned an error: {err}"))?
        .text()
        .map_err(|err| format!("Cannot read firmware release {release_id}: {err}"))?;
    let builds = parse_firmware_build_links(&index_url, &html, release_name, mode)?;

    if builds.is_empty() {
        return Err(format!(
            "Firmware release {release_id} contains no {mode} build pages. Select another release."
        ));
    }

    Ok(FirmwareBuildCatalog {
        release_name: percent_decode_catalog_segment(&release_id),
        release_id,
        source_url: index_url.to_string(),
        refreshed_at: now_seconds(),
        builds,
    })
}

fn inspect_firmware_release_blocking(
    release_id: &str,
    build_id: &str,
    mode: &str,
    models: &[String],
) -> Result<FirmwareReleaseInspection, String> {
    let release_id = validate_firmware_release_id(release_id)?;
    let build_id = validate_firmware_build_id(build_id)?;
    let release_url = firmware_release_url(&release_id)?;
    let build_url = firmware_build_url(&release_id, &build_id)?;
    let client = firmware_catalog_client()?;
    let html = client
        .get(build_url.clone())
        .send()
        .map_err(|err| format!("Cannot reach firmware build {release_id}/{build_id}: {err}"))?
        .error_for_status()
        .map_err(|err| format!("Firmware build {release_id}/{build_id} returned an error: {err}"))?
        .text()
        .map_err(|err| format!("Cannot read firmware build {release_id}/{build_id}: {err}"))?;
    let packages = parse_firmware_package_links(&release_url, &html)?;
    let preferred_extensions = firmware_package_extensions_for_mode(mode, models)?;
    let model_matches = map_firmware_packages_to_models(&packages, models, &preferred_extensions);

    Ok(FirmwareReleaseInspection {
        release_name: percent_decode_catalog_segment(&release_id),
        release_id,
        build_name: percent_decode_catalog_segment(&build_id),
        build_id,
        source_url: build_url.to_string(),
        refreshed_at: now_seconds(),
        packages,
        model_matches,
    })
}

fn map_firmware_packages_to_models(
    packages: &[FirmwareReleasePackage],
    models: &[String],
    preferred_extensions: &BTreeMap<String, String>,
) -> Vec<FirmwareReleaseModelMatch> {
    let normalized_models: BTreeSet<String> = models
        .iter()
        .map(|model| normalize_model_name(model))
        .filter(|model| !model.is_empty())
        .collect();
    normalized_models
        .into_iter()
        .map(|model| {
            let preferred_extension = required_firmware_package_extension(&model)
                .map(str::to_string)
                .or_else(|| preferred_extensions.get(&model).cloned());
            let candidates: Vec<String> = packages
                .iter()
                .filter(|package| firmware_package_matches_model(&package.file_name, &model))
                .filter(|package| {
                    preferred_extension.as_ref().map_or(true, |extension| {
                        Path::new(&package.file_name)
                            .extension()
                            .and_then(|value| value.to_str())
                            .is_some_and(|value| value.eq_ignore_ascii_case(extension))
                    })
                })
                .map(|package| package.file_name.clone())
                .collect();
            let (status, selected_file) = match candidates.as_slice() {
                [file_name] => ("Matched".to_string(), Some(file_name.clone())),
                [] => ("Missing".to_string(), None),
                _ => ("Ambiguous".to_string(), None),
            };
            FirmwareReleaseModelMatch {
                model,
                status,
                selected_file,
                candidates,
            }
        })
        .collect()
}

fn firmware_package_extensions_for_mode(
    mode: &str,
    models: &[String],
) -> Result<BTreeMap<String, String>, String> {
    let configs = read_firmware_configs()?;
    let config = configs
        .get(mode)
        .ok_or_else(|| format!("Firmware config mode not found: {mode}"))?;
    let mut extensions = BTreeMap::new();
    for model in models {
        let model = normalize_model_name(model);
        if let Some(extension) = required_firmware_package_extension(&model) {
            extensions.insert(model, extension.to_string());
            continue;
        }
        let suffix = firmware_suffix_for_model(config, &model, "");
        if let Some(extension) = Path::new(&suffix)
            .extension()
            .and_then(|value| value.to_str())
        {
            extensions.insert(model, extension.to_ascii_lowercase());
        }
    }
    Ok(extensions)
}

fn parse_firmware_release_links(
    base_url: &Url,
    html: &str,
) -> Result<Vec<FirmwareReleaseSummary>, String> {
    let active_releases =
        Regex::new(r"(?is)<h3[^>]*>\s*Active\s+Releases\s*</h3>\s*<ul[^>]*>(.*?)</ul>")
            .map_err(|err| format!("Cannot initialize active release parser: {err}"))?
            .captures(html)
            .and_then(|capture| capture.get(1))
            .map_or(html, |section| section.as_str());
    let mut seen = BTreeSet::new();
    let mut releases = Vec::new();
    for (href, label) in extract_html_links(active_releases)? {
        let Some(release_id) = direct_release_index_child(base_url, &href) else {
            continue;
        };
        if validate_firmware_release_id(&release_id).is_err() || !seen.insert(release_id.clone()) {
            continue;
        }
        let url = base_url
            .join(&format!("{release_id}/index.html"))
            .map_err(|err| format!("Invalid release link {release_id}: {err}"))?;
        releases.push(FirmwareReleaseSummary {
            id: release_id.clone(),
            name: if label.is_empty() {
                percent_decode_catalog_segment(&release_id)
            } else {
                label
            },
            url: url.to_string(),
        });
    }
    Ok(releases)
}

fn parse_firmware_build_links(
    release_index_url: &Url,
    html: &str,
    release_name: &str,
    mode: &str,
) -> Result<Vec<FirmwareBuildSummary>, String> {
    let release_url = release_index_url
        .join("./")
        .map_err(|err| format!("Invalid firmware release directory URL: {err}"))?;
    let mut seen = BTreeSet::new();
    let mut candidates = Vec::new();
    for (href, label) in extract_html_links(html)? {
        let href = href.trim();
        if href.is_empty() || href.starts_with('?') || href.starts_with('#') {
            continue;
        }
        let Some(joined) = release_index_url.join(href).ok() else {
            continue;
        };
        if joined.scheme() != release_url.scheme()
            || joined.host_str() != release_url.host_str()
            || joined.port_or_known_default() != release_url.port_or_known_default()
            || joined.query().is_some()
            || joined.fragment().is_some()
        {
            continue;
        }
        let Some(file_name) = joined.path().strip_prefix(release_url.path()) else {
            continue;
        };
        if file_name.is_empty()
            || file_name.contains('/')
            || !file_name.to_ascii_lowercase().ends_with(".html")
            || file_name.eq_ignore_ascii_case("index.html")
        {
            continue;
        }
        let encoded_build_id = &file_name[..file_name.len() - ".html".len()];
        let build_id = percent_decode_catalog_segment(encoded_build_id);
        if validate_firmware_build_id(&build_id).is_err() || !seen.insert(build_id.clone()) {
            continue;
        }
        candidates.push((build_id, label, joined));
    }

    let page_channel = explicit_firmware_channel(release_name)
        .map(str::to_string)
        .or(firmware_page_channel(html)?)
        .or_else(|| {
            consistent_firmware_channel(candidates.iter().map(|(_, label, _)| label.as_str()))
                .map(str::to_string)
        });

    Ok(candidates
        .into_iter()
        .filter(|(_, label, _)| firmware_build_matches_mode(label, page_channel.as_deref(), mode))
        .map(|(build_id, label, joined)| FirmwareBuildSummary {
            id: build_id.clone(),
            name: concise_firmware_build_label(&label, &build_id),
            url: joined.to_string(),
        })
        .collect())
}

fn firmware_page_channel(html: &str) -> Result<Option<String>, String> {
    let heading_pattern = Regex::new(r"(?is)<(?:h1|title)[^>]*>(.*?)</(?:h1|title)>")
        .map_err(|err| format!("Cannot initialize firmware page heading parser: {err}"))?;
    for capture in heading_pattern.captures_iter(html) {
        let heading = capture
            .get(1)
            .map(|value| html_text(value.as_str()))
            .unwrap_or_default();
        if let Some(channel) = explicit_firmware_channel(&heading) {
            return Ok(Some(channel.to_string()));
        }
    }
    Ok(None)
}

fn firmware_build_matches_mode(label: &str, page_channel: Option<&str>, mode: &str) -> bool {
    explicit_firmware_channel(label)
        .or(page_channel)
        .is_some_and(|channel| channel.eq_ignore_ascii_case(mode))
}

fn explicit_firmware_channel(value: &str) -> Option<&'static str> {
    let value = value.to_ascii_lowercase();
    let has_alpha = value.contains("alpha");
    let has_beta = value.contains("beta");
    match (has_alpha, has_beta) {
        (true, false) => Some("Alpha"),
        (false, true) => Some("Beta"),
        _ => None,
    }
}

fn consistent_firmware_channel<'a>(values: impl Iterator<Item = &'a str>) -> Option<&'static str> {
    let channels = values
        .filter_map(explicit_firmware_channel)
        .collect::<BTreeSet<_>>();
    (channels.len() == 1)
        .then(|| channels.into_iter().next())
        .flatten()
}

fn concise_firmware_build_label(label: &str, build_id: &str) -> String {
    let label = label.trim();
    if label.is_empty() {
        return build_id.to_string();
    }
    label
        .find(']')
        .map_or_else(|| label.to_string(), |index| label[..=index].to_string())
}

fn parse_firmware_package_links(
    release_url: &Url,
    html: &str,
) -> Result<Vec<FirmwareReleasePackage>, String> {
    let mut packages = BTreeMap::<String, FirmwareReleasePackage>::new();
    for href in extract_html_href_values(html)? {
        let Some(encoded_file_name) = direct_catalog_child(release_url, &href, false) else {
            continue;
        };
        let file_name = percent_decode_catalog_segment(&encoded_file_name);
        if validate_firmware_package_file_name(&file_name).is_err() {
            continue;
        }
        let url = release_url
            .join(&encoded_file_name)
            .map_err(|err| format!("Invalid firmware package link {file_name}: {err}"))?;
        packages
            .entry(file_name.clone())
            .or_insert(FirmwareReleasePackage {
                file_name,
                url: url.to_string(),
            });
    }
    Ok(packages.into_values().collect())
}

fn extract_html_href_values(html: &str) -> Result<Vec<String>, String> {
    let href_pattern = Regex::new(r#"(?i)href\s*=\s*(?:\"([^\"]*)\"|'([^']*)'|([^\s>]+))"#)
        .map_err(|err| format!("Cannot initialize release catalog parser: {err}"))?;
    Ok(href_pattern
        .captures_iter(html)
        .filter_map(|capture| {
            capture
                .get(1)
                .or_else(|| capture.get(2))
                .or_else(|| capture.get(3))
                .map(|value| decode_html_attribute(value.as_str()))
        })
        .collect())
}

fn extract_html_links(html: &str) -> Result<Vec<(String, String)>, String> {
    let link_pattern = Regex::new(
        r#"(?is)<a\b[^>]*?href\s*=\s*(?:\"([^\"]*)\"|'([^']*)'|([^\s>]+))[^>]*>(.*?)</a>"#,
    )
    .map_err(|err| format!("Cannot initialize firmware link parser: {err}"))?;
    Ok(link_pattern
        .captures_iter(html)
        .filter_map(|capture| {
            let href = capture
                .get(1)
                .or_else(|| capture.get(2))
                .or_else(|| capture.get(3))?;
            let label = capture.get(4).map_or("", |value| value.as_str());
            Some((decode_html_attribute(href.as_str()), html_text(label)))
        })
        .collect())
}

fn html_text(value: &str) -> String {
    let without_tags = Regex::new(r"(?is)<[^>]+>").map_or_else(
        |_| value.to_string(),
        |pattern| pattern.replace_all(value, " ").into_owned(),
    );
    decode_html_attribute(&without_tags)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn decode_html_attribute(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

fn direct_catalog_child(base_url: &Url, href: &str, expect_directory: bool) -> Option<String> {
    let href = href.trim();
    if href.is_empty() || href.starts_with('?') || href.starts_with('#') {
        return None;
    }
    let joined = base_url.join(href).ok()?;
    if joined.scheme() != base_url.scheme()
        || joined.host_str() != base_url.host_str()
        || joined.port_or_known_default() != base_url.port_or_known_default()
        || joined.query().is_some()
        || joined.fragment().is_some()
    {
        return None;
    }
    let relative = joined.path().strip_prefix(base_url.path())?;
    let relative = if expect_directory {
        relative.strip_suffix('/')?
    } else {
        if relative.ends_with('/') {
            return None;
        }
        relative
    };
    if relative.is_empty() || relative.contains('/') {
        return None;
    }
    Some(relative.to_string())
}

fn direct_release_index_child(base_url: &Url, href: &str) -> Option<String> {
    let href = href.trim();
    if href.is_empty() || href.starts_with('?') || href.starts_with('#') {
        return None;
    }
    let joined = base_url.join(href).ok()?;
    if joined.scheme() != base_url.scheme()
        || joined.host_str() != base_url.host_str()
        || joined.port_or_known_default() != base_url.port_or_known_default()
        || joined.query().is_some()
        || joined.fragment().is_some()
    {
        return None;
    }
    let relative = joined.path().strip_prefix(base_url.path())?;
    let release_id = relative.strip_suffix("/index.html")?;
    if release_id.is_empty() || release_id.contains('/') {
        return None;
    }
    Some(release_id.to_string())
}

fn percent_decode_catalog_segment(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (
                (bytes[index + 1] as char).to_digit(16),
                (bytes[index + 2] as char).to_digit(16),
            ) {
                decoded.push(((high << 4) | low) as u8);
                index += 3;
                continue;
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn firmware_package_matches_model(file_name: &str, model: &str) -> bool {
    let file_name = file_name.to_ascii_lowercase();
    let model = normalize_model_name(model);
    file_name
        .strip_prefix(&model)
        .and_then(|suffix| suffix.chars().next())
        .is_some_and(|separator| matches!(separator, '_' | '-' | '.'))
}

fn validate_firmware_plan_blocking(
    request: FirmwarePlanRequest,
) -> Result<FirmwarePreflightResult, String> {
    if request.firmware_version.trim().is_empty() {
        return Err("Firmware version is required.".to_string());
    }

    if request.package_overrides.is_none() && request.firmware_suffix.trim().is_empty() {
        return Err("Firmware suffix or package name is required.".to_string());
    }

    let (_mode_config, group, receivers) = resolve_firmware_receivers(&request)?;
    if receivers.is_empty() {
        return Err("Select at least one receiver for preflight.".to_string());
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(8))
        .connect_timeout(Duration::from_secs(4))
        .build()
        .map_err(|err| format!("HTTP client error: {err}"))?;

    let mut rows = Vec::with_capacity(receivers.len());
    for rx in receivers {
        rows.push(preflight_firmware_receiver(&client, &rx));
    }

    let ready_count = rows.iter().filter(|row| row.status == "Ready").count();
    let warning_count = rows.iter().filter(|row| row.status == "Warning").count();
    let failed_count = rows.iter().filter(|row| row.status == "Failed").count();

    Ok(FirmwarePreflightResult {
        mode: request.mode,
        group,
        group_name: firmware_group_name(group).to_string(),
        firmware_version: request.firmware_version.trim().to_string(),
        firmware_suffix: if request.package_overrides.is_some() {
            "Release catalog".to_string()
        } else {
            request.firmware_suffix.trim().to_string()
        },
        ready_count,
        warning_count,
        failed_count,
        receivers: rows,
    })
}

fn dry_run_single_firmware_update_blocking(
    request: FirmwareSingleUpdateRequest,
) -> Result<FirmwareSingleDryRunResult, String> {
    if request.receiver_ip.trim().is_empty() {
        return Err("Receiver IP is required.".to_string());
    }

    let source_mode = request
        .source_mode
        .as_deref()
        .unwrap_or("release")
        .trim()
        .to_ascii_lowercase();
    let uses_local_package = source_mode == "local";

    if !uses_local_package && request.firmware_version.trim().is_empty() {
        return Err("Firmware version is required for release server dry run.".to_string());
    }

    if !uses_local_package && request.firmware_suffix.trim().is_empty() {
        return Err("Firmware suffix is required for release server dry run.".to_string());
    }

    let mut steps = Vec::new();
    push_workflow_step(
        &mut steps,
        "Queued",
        "Completed",
        "Receiver selected for single receiver dry run.",
    );

    let receiver = resolve_single_firmware_receiver(&request, uses_local_package)?;

    push_workflow_step(
        &mut steps,
        "Preflight",
        "Running",
        "Checking receiver credentials, current firmware data, and install state.",
    );

    let receiver_client = Client::builder()
        .timeout(Duration::from_secs(8))
        .connect_timeout(Duration::from_secs(4))
        .build()
        .map_err(|err| format!("HTTP client error: {err}"))?;
    let preflight = preflight_firmware_receiver(&receiver_client, &receiver);
    let preflight_ok = preflight.can_update;
    push_workflow_step(
        &mut steps,
        "Preflight",
        if preflight_ok { "Completed" } else { "Failed" },
        &preflight.message,
    );

    push_workflow_step(
        &mut steps,
        "PackageCheck",
        "Running",
        "Checking target firmware package availability.",
    );

    let package_ok = if uses_local_package {
        push_workflow_step(
            &mut steps,
            "PackageCheck",
            "Completed",
            "Local firmware package is staged in the UI. Remote package check skipped.",
        );
        true
    } else {
        let package_client = Client::builder()
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|err| format!("HTTP client error: {err}"))?;
        match check_firmware_package_available(&package_client, &receiver.target_url) {
            Ok(()) => {
                push_workflow_step(
                    &mut steps,
                    "PackageCheck",
                    "Completed",
                    "Target firmware package is reachable.",
                );
                true
            }
            Err(err) => {
                push_workflow_step(&mut steps, "PackageCheck", "Failed", &err);
                false
            }
        }
    };

    for state in [
        "DownloadPackage",
        "ChangePassword",
        "UploadFirmware",
        "WaitForInstall",
        "RestorePassword",
        "VerifyResult",
    ] {
        push_workflow_step(
            &mut steps,
            state,
            "Skipped",
            "Dry run only. No receiver state was changed.",
        );
    }

    let status = if preflight_ok && package_ok {
        "Prepared"
    } else {
        "Failed"
    };

    Ok(FirmwareSingleDryRunResult {
        ip: receiver.credential.addr,
        model: receiver.model,
        target_file: receiver.target_file,
        target_url: receiver.target_url,
        receiver_name: preflight.receiver_name,
        firmware_version: preflight.firmware_version,
        uptime_seconds: preflight.uptime_seconds,
        status: status.to_string(),
        steps,
    })
}

fn run_single_local_firmware_update_job(
    single_jobs: Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    lease: FirmwareJobLease,
    job_id: String,
    request: FirmwareSingleLocalUpdateRequest,
) {
    let control = lease.control();
    match catch_unwind(AssertUnwindSafe(|| {
        run_single_local_firmware_update_job_inner(&single_jobs, &control, &job_id, request)
    })) {
        Ok(Ok(())) => {}
        Ok(Err(err)) => {
            complete_worker_error(&single_jobs, &control, &job_id, &err);
        }
        Err(_) => {
            let message = "Firmware update stopped unexpectedly. Password recovery was attempted.";
            update_single_job_step(&single_jobs, &job_id, "Failed", "Failed", "Failed", message);
            complete_single_job(&single_jobs, &job_id, "Failed", message);
        }
    }
}

fn complete_worker_error(
    single_jobs: &Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    control: &FirmwareJobControl,
    job_id: &str,
    error: &str,
) {
    let (phase, status, message) = if control.is_cancel_requested() {
        (
            "Cancelled",
            "Cancelled",
            "Firmware update was cancelled. Password recovery was attempted before releasing the receiver.",
        )
    } else {
        ("Failed", "Failed", error)
    };
    update_single_job_step(single_jobs, job_id, phase, status, status, message);
    complete_single_job(single_jobs, job_id, status, message);
}

fn ensure_firmware_job_not_cancelled(control: &FirmwareJobControl) -> Result<(), String> {
    if control.is_cancel_requested() {
        Err("Firmware update cancellation was requested.".to_string())
    } else {
        Ok(())
    }
}

fn run_single_release_firmware_update_job(
    single_jobs: Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    lease: FirmwareJobLease,
    job_id: String,
    request: FirmwareSingleUpdateRequest,
) {
    let control = lease.control();
    match catch_unwind(AssertUnwindSafe(|| {
        run_single_release_firmware_update_job_inner(&single_jobs, &control, &job_id, request)
    })) {
        Ok(Ok(())) => {}
        Ok(Err(err)) => {
            complete_worker_error(&single_jobs, &control, &job_id, &err);
        }
        Err(_) => {
            let message = "Firmware update stopped unexpectedly. Password recovery was attempted.";
            update_single_job_step(&single_jobs, &job_id, "Failed", "Failed", "Failed", message);
            complete_single_job(&single_jobs, &job_id, "Failed", message);
        }
    }
}

fn run_single_release_firmware_update_job_inner(
    single_jobs: &Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    control: &FirmwareJobControl,
    job_id: &str,
    request: FirmwareSingleUpdateRequest,
) -> Result<(), String> {
    ensure_firmware_job_not_cancelled(control)?;
    let receiver = resolve_single_firmware_receiver(&request, false)?;

    update_single_job_step(
        single_jobs,
        job_id,
        "DownloadPackage",
        "Running",
        "Running",
        "Downloading firmware package from release server.",
    );

    let package_path = download_release_firmware_package(
        &receiver.target_url,
        &receiver.target_file,
        &receiver.credential.addr,
        job_id,
    )?;
    ensure_firmware_job_not_cancelled(control)?;

    update_single_job_step(
        single_jobs,
        job_id,
        "DownloadPackage",
        "Completed",
        "Running",
        "Release server firmware package is ready for receiver upload.",
    );

    let local_request = FirmwareSingleLocalUpdateRequest {
        batch_id: request.batch_id.clone(),
        receiver_ip: receiver.credential.addr,
        receiver_model: Some(receiver.model),
        local_file_name: receiver.target_file,
        local_file_path: package_path.display().to_string(),
    };

    run_single_local_firmware_update_job_inner(single_jobs, control, job_id, local_request)
}

fn run_single_local_firmware_update_job_inner(
    single_jobs: &Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    control: &FirmwareJobControl,
    job_id: &str,
    request: FirmwareSingleLocalUpdateRequest,
) -> Result<(), String> {
    ensure_firmware_job_not_cancelled(control)?;
    let access = receiver_password::operation_lock(request.receiver_ip.trim())?;
    let _access = access
        .lock()
        .map_err(|_| "Receiver operation lock is unavailable.")?;
    password_recovery_notices::ensure_ready(request.receiver_ip.trim())?;
    let file_path = Path::new(request.local_file_path.trim());
    if !file_path.is_file() {
        return Err("Local firmware package path is not readable.".to_string());
    }

    let file_name = request.local_file_name.trim().to_string();
    validate_local_firmware_file_name(&file_name)?;

    let dry_request = FirmwareSingleUpdateRequest {
        mode: "Auto".to_string(),
        batch_id: request.batch_id.clone(),
        receiver_ip: request.receiver_ip,
        receiver_model: request.receiver_model,
        firmware_version: String::new(),
        firmware_suffix: String::new(),
        package_file: None,
        source_mode: Some("local".to_string()),
        local_file_name: Some(file_name.clone()),
    };
    let receiver = resolve_single_firmware_receiver(&dry_request, true)?;

    let firmware_security_mode =
        check_initial_firmware_password(single_jobs, job_id, &receiver.credential)?;

    update_single_job_step(
        single_jobs,
        job_id,
        "Preflight",
        "Running",
        "Running",
        "Checking receiver credentials, firmware state, uptime and current firmware.",
    );

    let receiver_client = Client::builder()
        .timeout(Duration::from_secs(8))
        .connect_timeout(Duration::from_secs(4))
        .build()
        .map_err(|err| format!("HTTP client error: {err}"))?;
    let preflight = preflight_firmware_receiver(&receiver_client, &receiver);
    update_single_job_metadata(
        single_jobs,
        job_id,
        preflight.receiver_name.clone(),
        preflight.firmware_version.clone(),
        preflight.uptime_seconds,
    );
    update_single_job_step(
        single_jobs,
        job_id,
        "Preflight",
        if preflight.can_update {
            "Completed"
        } else {
            "Failed"
        },
        if preflight.can_update {
            "Running"
        } else {
            "Failed"
        },
        &preflight.message,
    );

    if !preflight.can_update {
        complete_single_job(single_jobs, job_id, "Failed", &preflight.message);
        return Ok(());
    }

    update_single_job_step(
        single_jobs,
        job_id,
        "UpdateSafety",
        "Running",
        "Running",
        "Checking the configured convergence restart window.",
    );
    match ensure_update_safety_with_known_uptime(
        &receiver.credential.addr,
        preflight.uptime_seconds,
    ) {
        Ok(evaluation) => update_single_job_step(
            single_jobs,
            job_id,
            "UpdateSafety",
            "Completed",
            "Running",
            &evaluation.message,
        ),
        Err(error) => {
            update_single_job_step(
                single_jobs,
                job_id,
                "UpdateSafety",
                "Failed",
                "Failed",
                &error,
            );
            complete_single_job(single_jobs, job_id, "Failed", &error);
            return Ok(());
        }
    }

    update_single_job_step(
        single_jobs,
        job_id,
        "CloneConfig",
        "Running",
        "Running",
        "Creating and downloading receiver configuration clone file.",
    );
    match create_receiver_clone_backup(&receiver_client, &receiver.credential, job_id) {
        Ok(clone_file) => {
            update_single_job_clone_file(single_jobs, job_id, &clone_file);
            update_single_job_step(
                single_jobs,
                job_id,
                "CloneConfig",
                "Completed",
                "Running",
                &format!(
                    "Receiver configuration clone file saved: {}.",
                    clone_file.file_name
                ),
            );
        }
        Err(err) => {
            update_single_job_step(single_jobs, job_id, "CloneConfig", "Failed", "Failed", &err);
            complete_single_job(single_jobs, job_id, "Failed", &err);
            return Ok(());
        }
    }

    ensure_firmware_job_not_cancelled(control)?;

    if is_firmware_downgrade(preflight.firmware_version.as_deref(), &file_name) {
        let current_version = preflight.firmware_version.as_deref().unwrap_or("unknown");
        let message = format!(
            "Downgrade approval is required for {} (current FW {}, target {}). Open the receiver Firmware page, confirm the downgrade there, then continue or cancel this receiver in QA Receiver Center.",
            receiver.credential.addr, current_version, file_name
        );
        set_single_job_downgrade_gate(single_jobs, job_id, &message);

        match wait_for_downgrade_decision(single_jobs, control, job_id) {
            Ok(true) => update_single_job_step(
                single_jobs,
                job_id,
                "DowngradeApproval",
                "Completed",
                "Running",
                "Downgrade was approved by the operator. Continuing before any receiver password change.",
            ),
            Ok(false) => {
                let cancelled_message =
                    "Downgrade was cancelled by the operator. No receiver password was changed.";
                update_single_job_step(
                    single_jobs,
                    job_id,
                    "DowngradeApproval",
                    "Cancelled",
                    "Failed",
                    cancelled_message,
                );
                complete_single_job(single_jobs, job_id, "Failed", cancelled_message);
                return Ok(());
            }
            Err(err) => {
                update_single_job_step(
                    single_jobs,
                    job_id,
                    "DowngradeApproval",
                    "Failed",
                    "Failed",
                    &err,
                );
                complete_single_job(single_jobs, job_id, "Failed", &err);
                return Ok(());
            }
        }
    }

    let temp_password = firmware_temp_password()?;
    let operation_password;
    let mut password_recovery = None;
    if firmware_security_mode == receiver_password::FirmwareSecurityMode::PasswordProtected {
        let recovery_receiver = receiver.credential.clone();
        let recovery_temp_password = temp_password.clone();
        let recovery_jobs = Arc::clone(single_jobs);
        let recovery_job_id = job_id.to_string();
        password_recovery_notices::arm(&receiver.credential.addr, job_id)?;
        // Arm recovery before the write: a timeout may hide a successful password change.
        password_recovery = Some(PasswordRecoveryGuard::new(move || {
            update_single_job_step(
                &recovery_jobs,
                &recovery_job_id,
                "RestorePassword",
                "Running",
                "Running",
                "Restoring and verifying the configured receiver password.",
            );
            let result = restore_receiver_password_with_retry(&recovery_receiver, &recovery_temp_password)
                .and_then(|()| password_recovery_notices::clear(&recovery_receiver.addr).map_err(|error| {
                    format!("PASSWORD RECOVERY REQUIRED for {}. The password was verified, but the recovery record could not be cleared: {error}", recovery_receiver.addr)
                }));
            update_single_job_step(
                &recovery_jobs,
                &recovery_job_id,
                "RestorePassword",
                if result.is_ok() {
                    "Completed"
                } else {
                    "Failed"
                },
                "Running",
                result
                    .as_ref()
                    .err()
                    .map(String::as_str)
                    .unwrap_or("Configured receiver password restored and verified."),
            );
            result
        }));
        update_single_job_step(
            single_jobs,
            job_id,
            "ChangePassword",
            "Running",
            "Running",
            "Changing receiver password to the temporary firmware password.",
        );
        match activate_temporary_firmware_password(&receiver.credential, &temp_password) {
            Ok(password_changed) => update_single_job_step(
                single_jobs,
                job_id,
                "ChangePassword",
                "Completed",
                "Running",
                if password_changed {
                    "Temporary password is active and verified for firmware upload."
                } else {
                    "Temporary password was already active. Password recovery remains armed."
                },
            ),
            Err(err) => {
                update_single_job_step(
                    single_jobs,
                    job_id,
                    "ChangePassword",
                    "Failed",
                    "Failed",
                    &err,
                );
                let recovery_result = password_recovery
                    .as_mut()
                    .expect("protected receiver recovery guard")
                    .restore_now();
                let message = recovery_result.err().unwrap_or(err);
                complete_single_job(single_jobs, job_id, "Failed", &message);
                return Ok(());
            }
        }
        operation_password = temp_password.clone();
    } else {
        update_single_job_step(
            single_jobs,
            job_id,
            "ChangePassword",
            "Skipped",
            "Running",
            "Receiver security is disabled. No temporary password was set.",
        );
        operation_password = receiver.credential.pw.clone();
    }

    ensure_firmware_job_not_cancelled(control)?;

    update_single_job_step(
        single_jobs,
        job_id,
        "UploadFirmware",
        "Running",
        "Running",
        "Uploading local firmware package to receiver failsafe region.",
    );
    let upload_monitor_stop = Arc::new(AtomicBool::new(false));
    let upload_monitor = {
        let monitor_stop = Arc::clone(&upload_monitor_stop);
        let monitor_jobs = Arc::clone(single_jobs);
        let monitor_rx = receiver.credential.clone();
        let monitor_password = operation_password.clone();
        let monitor_job_id = job_id.to_string();
        thread::spawn(move || {
            monitor_receiver_firmware_status_until_stop(
                &monitor_rx,
                &monitor_password,
                &monitor_jobs,
                &monitor_job_id,
                "UploadFirmware",
                monitor_stop,
            );
        })
    };
    let upload_result = upload_local_firmware_package(
        &receiver.credential,
        file_path,
        &file_name,
        &operation_password,
    );
    upload_monitor_stop.store(true, Ordering::SeqCst);
    let _ = upload_monitor.join();
    let mut final_status = "Completed".to_string();
    match upload_result {
        Ok(()) => update_single_job_step(
            single_jobs,
            job_id,
            "UploadFirmware",
            "Completed",
            "Running",
            "Firmware package upload request completed.",
        ),
        Err(err) => {
            final_status = "Failed".to_string();
            update_single_job_step(
                single_jobs,
                job_id,
                "UploadFirmware",
                "Failed",
                "Failed",
                &err,
            );
        }
    }

    if final_status == "Completed" {
        update_single_job_step(
            single_jobs,
            job_id,
            "MonitorInstall",
            "Running",
            "Running",
            "Monitoring receiver firmware_status/fw_status until install cycle completes.",
        );
        // After upload starts, cancellation drains the active receiver through reboot and password recovery.
        match wait_for_receiver_install_live(
            &receiver.credential,
            &operation_password,
            single_jobs,
            control,
            job_id,
            preflight.firmware_version.as_deref(),
            preflight.uptime_seconds,
        ) {
            Ok(()) => update_single_job_step(
                single_jobs,
                job_id,
                "MonitorInstall",
                "Completed",
                "Running",
                "Receiver returned after firmware install cycle.",
            ),
            Err(err) => {
                let cancelled = control.is_cancel_requested();
                final_status = if cancelled { "Cancelled" } else { "Failed" }.to_string();
                update_single_job_step(
                    single_jobs,
                    job_id,
                    "MonitorInstall",
                    if cancelled { "Cancelled" } else { "Failed" },
                    if cancelled { "Cancelling" } else { "Failed" },
                    &err,
                );
            }
        }
    } else {
        update_single_job_step(
            single_jobs,
            job_id,
            "MonitorInstall",
            "Skipped",
            "Failed",
            "Upload did not complete, install monitoring was skipped.",
        );
    }

    if let Some(password_recovery) = password_recovery.as_mut() {
        update_single_job_step(
            single_jobs,
            job_id,
            "RestorePassword",
            "Running",
            match final_status.as_str() {
                "Completed" => "Running",
                "Cancelled" => "Cancelling",
                _ => "Failed",
            },
            "Restoring receiver password from receiver configuration.",
        );
        match password_recovery.restore_now() {
            Ok(()) => update_single_job_step(
                single_jobs,
                job_id,
                "RestorePassword",
                "Completed",
                match final_status.as_str() {
                    "Completed" => "Running",
                    "Cancelled" => "Cancelling",
                    _ => "Failed",
                },
                "Receiver password was restored to the configured value.",
            ),
            Err(err) => {
                final_status = "Failed".to_string();
                update_single_job_step(
                    single_jobs,
                    job_id,
                    "RestorePassword",
                    "Failed",
                    "Failed",
                    &err,
                );
            }
        }
    } else {
        update_single_job_step(
            single_jobs,
            job_id,
            "RestorePassword",
            "Skipped",
            match final_status.as_str() {
                "Completed" => "Running",
                "Cancelled" => "Cancelling",
                _ => "Failed",
            },
            "Receiver security is disabled. No password was changed, so password recovery was not required.",
        );
    }

    if final_status == "Completed" {
        update_single_job_step(
            single_jobs,
            job_id,
            "VerifyResult",
            "Running",
            "Running",
            "Verifying receiver after password restore.",
        );
        let verify_client = Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(4))
            .build()
            .map_err(|err| format!("HTTP client error: {err}"))?;
        let verify = preflight_firmware_receiver(&verify_client, &receiver);
        if !verify.can_update {
            final_status = "Failed".to_string();
        }
        update_single_job_metadata(
            single_jobs,
            job_id,
            verify.receiver_name.clone(),
            verify.firmware_version.clone(),
            verify.uptime_seconds,
        );
        update_single_job_step(
            single_jobs,
            job_id,
            "VerifyResult",
            if verify.can_update {
                "Completed"
            } else {
                "Failed"
            },
            "Running",
            &verify.message,
        );
    } else {
        update_single_job_step(
            single_jobs,
            job_id,
            "VerifyResult",
            "Skipped",
            if final_status == "Cancelled" {
                "Cancelling"
            } else {
                "Failed"
            },
            if final_status == "Cancelled" {
                "Update was cancelled after password recovery. Result verification was skipped."
            } else {
                "Update did not complete cleanly, verification was skipped."
            },
        );
    }

    let message = match final_status.as_str() {
        "Completed" => "Local firmware update completed.",
        "Cancelled" => {
            "Firmware update was cancelled and password recovery finished. The receiver reservation was released."
        }
        _ => "Local firmware update failed. Check failed step details.",
    };
    complete_single_job(single_jobs, job_id, &final_status, message);
    Ok(())
}

fn resolve_single_firmware_receiver(
    request: &FirmwareSingleUpdateRequest,
    uses_local_package: bool,
) -> Result<FirmwarePlanReceiver, String> {
    let ip = request.receiver_ip.trim();
    if ip.parse::<Ipv4Addr>().is_err() {
        return Err(format!("Invalid receiver IP: {ip}"));
    }

    let credential = read_receivers(GROUPS_JSON_PATH)?
        .into_iter()
        .find(|rx| rx.addr == ip)
        .ok_or_else(|| format!("Receiver credentials not found for {ip}"))?;
    let configs = read_firmware_configs()?;
    let requested_mode = request.mode.trim();
    let mode_candidates: Vec<(&String, &FirmwareConfigMode)> =
        if requested_mode.is_empty() || requested_mode.eq_ignore_ascii_case("auto") {
            configs.iter().collect()
        } else {
            vec![(
                configs
                    .get_key_value(requested_mode)
                    .map(|(mode, _)| mode)
                    .ok_or_else(|| format!("Firmware config mode not found: {requested_mode}"))?,
                configs
                    .get(requested_mode)
                    .ok_or_else(|| format!("Firmware config mode not found: {requested_mode}"))?,
            )]
        };

    let matched_config = mode_candidates
        .into_iter()
        .find_map(|(_mode, config)| config.receiver_models.get(ip).map(|model| (config, model)));
    let model_override = request
        .receiver_model
        .as_deref()
        .map(normalize_model_name)
        .filter(|value| !value.is_empty());
    let exact_package = request
        .package_file
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let (model, target_file, target_url) = if let Some((config, model)) = matched_config {
        let model = model_override.unwrap_or_else(|| normalize_model_name(model));
        if uses_local_package {
            let file_name = request
                .local_file_name
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    "Local firmware file is required for local package dry run.".to_string()
                })?;
            validate_firmware_package_for_model(file_name, &model)?;
            (model, file_name.to_string(), format!("local://{file_name}"))
        } else {
            let (target_file, target_url) = if let Some(package_file) = exact_package {
                validate_firmware_package_for_model(package_file, &model)?;
                firmware_target_for_exact_package(&request.firmware_version, package_file)?
            } else {
                firmware_target_for_model(
                    config,
                    &request.firmware_version,
                    &request.firmware_suffix,
                    &model,
                )
            };
            (model, target_file, target_url)
        }
    } else if uses_local_package {
        let file_name = request
            .local_file_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                "Local firmware file is required for local package dry run.".to_string()
            })?;
        let model = model_override.unwrap_or_else(|| "unknown".to_string());
        validate_firmware_package_for_model(file_name, &model)?;
        (model, file_name.to_string(), format!("local://{file_name}"))
    } else if let Some(model) = model_override {
        if let Some(package_file) = exact_package {
            validate_firmware_package_for_model(package_file, &model)?;
            let (target_file, target_url) =
                firmware_target_for_exact_package(&request.firmware_version, package_file)?;
            (model, target_file, target_url)
        } else {
            let suffix =
                firmware_suffix_with_required_extension(&model, request.firmware_suffix.trim());
            let filename = format!("{model}{suffix}");
            validate_firmware_package_for_model(&filename, &model)?;
            let url = format!(
                "{}/{}/{}",
                FIRMWARE_RELEASE_BASE_URL,
                request.firmware_version.trim(),
                filename
            );
            (model, filename, url)
        }
    } else {
        return Err(format!(
            "Receiver {ip} is not assigned to Alpha or Beta firmware config. Select receiver model for release server dry run."
        ));
    };

    Ok(FirmwarePlanReceiver {
        credential,
        model,
        target_file,
        target_url,
        local_package_path: None,
    })
}

fn validate_local_firmware_file_name(file_name: &str) -> Result<(), String> {
    let lower = file_name.to_ascii_lowercase();
    if lower.ends_with(".img") || lower.ends_with(".timg") {
        Ok(())
    } else {
        Err("Only .img and .timg firmware packages are supported.".to_string())
    }
}

fn validate_firmware_package_for_model(file_name: &str, model: &str) -> Result<(), String> {
    validate_local_firmware_file_name(file_name)?;
    let Some(required_extension) = required_firmware_package_extension(model) else {
        return Ok(());
    };
    let actual_extension = Path::new(file_name)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if actual_extension.eq_ignore_ascii_case(required_extension) {
        Ok(())
    } else {
        Err(format!(
            "Receiver model {} requires an .{} firmware package.",
            normalize_model_name(model).to_ascii_uppercase(),
            required_extension
        ))
    }
}

fn validate_firmware_package_name_for_model(file_name: &str, model: &str) -> Result<(), String> {
    let normalized_model = normalize_model_name(model);
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let contains_model = stem
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|token| token == normalized_model);
    if contains_model {
        Ok(())
    } else {
        Err(format!(
            "Firmware package {} does not contain receiver model {} in its file name.",
            file_name,
            normalized_model.to_ascii_uppercase()
        ))
    }
}

fn pick_local_firmware_package_blocking() -> Result<Option<LocalFirmwarePackage>, String> {
    #[cfg(target_os = "windows")]
    {
        let script = r#"
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.OpenFileDialog
$dialog.Title = 'Select firmware package'
$dialog.Filter = 'Firmware packages (*.img;*.timg)|*.img;*.timg|All files (*.*)|*.*'
$dialog.Multiselect = $false
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
  [Console]::Out.Write($dialog.FileName)
}
"#;
        let output = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-STA",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                script,
            ])
            .output()
            .map_err(|err| format!("Cannot open firmware file picker: {err}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if stderr.is_empty() {
                "Firmware file picker failed.".to_string()
            } else {
                format!("Firmware file picker failed: {stderr}")
            });
        }

        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if path.is_empty() {
            return Ok(None);
        }

        Ok(Some(local_firmware_package_from_path(&path)?))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Native firmware package picker is not implemented for this OS yet.".to_string())
    }
}

fn pick_local_firmware_packages_blocking() -> Result<Vec<LocalFirmwarePackage>, String> {
    #[cfg(target_os = "windows")]
    {
        let script = r#"
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.OpenFileDialog
$dialog.Title = 'Select firmware packages'
$dialog.Filter = 'Firmware packages (*.img;*.timg)|*.img;*.timg|All files (*.*)|*.*'
$dialog.Multiselect = $true
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
  $dialog.FileNames | ForEach-Object { [Console]::Out.WriteLine($_) }
}
"#;
        let output = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-STA",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                script,
            ])
            .output()
            .map_err(|err| format!("Cannot open firmware file picker: {err}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if stderr.is_empty() {
                "Firmware file picker failed.".to_string()
            } else {
                format!("Firmware file picker failed: {stderr}")
            });
        }

        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(local_firmware_package_from_path)
            .collect()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Native multi-package picker is not implemented for this OS yet.".to_string())
    }
}

fn local_firmware_package_from_path(path: &str) -> Result<LocalFirmwarePackage, String> {
    let package_path = Path::new(path);
    if !package_path.is_file() {
        return Err("Selected firmware package path is not readable.".to_string());
    }

    let name = package_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Selected firmware package name is invalid.".to_string())?
        .to_string();
    validate_local_firmware_file_name(&name)?;
    let size = fs::metadata(package_path)
        .map_err(|err| format!("Cannot inspect selected firmware package: {err}"))?
        .len();

    Ok(LocalFirmwarePackage {
        name,
        path: path.to_string(),
        size,
    })
}

fn push_workflow_step(
    steps: &mut Vec<FirmwareWorkflowStep>,
    state: &str,
    status: &str,
    message: &str,
) {
    steps.push(FirmwareWorkflowStep {
        state: state.to_string(),
        status: status.to_string(),
        message: message.to_string(),
    });
}

fn check_firmware_package_available(client: &Client, url: &str) -> Result<(), String> {
    let head_status = client
        .head(url)
        .send()
        .map(|response| response.status())
        .map_err(|err| format!("Cannot check firmware package: {}", clean_error(err)))?;

    if head_status.is_success() {
        return Ok(());
    }

    let get_status = client
        .get(url)
        .header("Range", "bytes=0-0")
        .send()
        .map(|response| response.status())
        .map_err(|err| format!("Cannot check firmware package: {}", clean_error(err)))?;

    if get_status.is_success() {
        Ok(())
    } else {
        Err(format!(
            "Firmware package is not reachable. HEAD returned {head_status}, GET returned {get_status}."
        ))
    }
}

fn download_release_firmware_package(
    url: &str,
    file_name: &str,
    receiver_ip: &str,
    job_id: &str,
) -> Result<PathBuf, String> {
    validate_local_firmware_file_name(file_name)?;

    let client = Client::builder()
        .timeout(Duration::from_secs(900))
        .connect_timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| format!("HTTP client error: {err}"))?;
    check_firmware_package_available(&client, url)?;

    let response = client
        .get(url)
        .send()
        .map_err(|err| format!("Cannot download release package: {}", clean_error(err)))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "Cannot download release package {}: HTTP {}.",
            file_name,
            status.as_u16()
        ));
    }

    let bytes = response
        .bytes()
        .map_err(|err| format!("Cannot read release package download: {}", clean_error(err)))?;
    if bytes.is_empty() {
        return Err(format!(
            "Release package {} downloaded as an empty file.",
            file_name
        ));
    }

    let safe_ip = sanitize_clone_name_part(receiver_ip);
    let dir = qa_receiver_center_data_dir()?
        .join("firmware-cache")
        .join(safe_ip)
        .join(job_id);
    fs::create_dir_all(&dir).map_err(|err| {
        format!(
            "Cannot create firmware package cache directory {}: {err}",
            dir.display()
        )
    })?;
    let path = dir.join(file_name);
    fs::write(&path, &bytes).map_err(|err| {
        format!(
            "Cannot write release package cache file {}: {err}",
            path.display()
        )
    })?;

    Ok(path)
}

fn resolve_firmware_receivers(
    request: &FirmwarePlanRequest,
) -> Result<(FirmwareConfigMode, u8, Vec<FirmwarePlanReceiver>), String> {
    let configs = read_firmware_configs()?;
    let mode_config = configs
        .get(&request.mode)
        .cloned()
        .ok_or_else(|| format!("Firmware config mode not found: {}", request.mode))?;
    let group = firmware_mode_group(&request.mode)?;
    let credentials = read_receivers(GROUPS_JSON_PATH)?;

    let selected: Vec<String> = request
        .receiver_ips
        .iter()
        .map(|ip| ip.trim().to_string())
        .filter(|ip| !ip.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    for ip in &selected {
        if ip.parse::<Ipv4Addr>().is_err() {
            return Err(format!("Invalid receiver IP: {ip}"));
        }

        let receiver = credentials
            .iter()
            .find(|receiver| receiver.addr == *ip)
            .ok_or_else(|| format!("Receiver {ip} was not found in qa-receivers.json."))?;
        if receiver.group != group {
            return Err(format!(
                "Receiver {ip} belongs to {} in qa-receivers.json, not {}.",
                firmware_group_name(receiver.group),
                firmware_group_name(group)
            ));
        }
    }

    let mut receivers = Vec::new();
    for ip in selected {
        let credential = credentials
            .iter()
            .find(|rx| rx.addr == ip)
            .cloned()
            .ok_or_else(|| format!("Receiver credentials not found for {ip}"))?;
        let model = firmware_model_for_receiver(&configs, &request.mode, &ip).ok_or_else(|| {
            format!(
                "Receiver model not found for {ip}. Assign its firmware model before starting the update."
            )
        })?;
        let (target_file, target_url, local_package_path) =
            if let Some(local_paths) = &request.local_package_paths {
                let package_path = local_paths
                    .iter()
                    .find(|(configured_model, _)| normalize_model_name(configured_model) == model)
                    .map(|(_, path)| PathBuf::from(path))
                    .ok_or_else(|| {
                        format!(
                            "Local firmware package is missing for receiver model {}.",
                            model.to_ascii_uppercase()
                        )
                    })?;
                if !package_path.is_file() {
                    return Err(format!(
                        "Local firmware package for {} is not readable: {}",
                        model.to_ascii_uppercase(),
                        package_path.display()
                    ));
                }
                let package_file = package_path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| "Local firmware package name is invalid.".to_string())?
                    .to_string();
                validate_firmware_package_for_model(&package_file, &model)?;
                validate_firmware_package_name_for_model(&package_file, &model)?;
                (
                    package_file.clone(),
                    format!("local://{package_file}"),
                    Some(package_path),
                )
            } else if let Some(package_overrides) = &request.package_overrides {
                let package_file = package_overrides.get(&model).ok_or_else(|| {
                    format!(
                    "Release catalog did not resolve a firmware package for receiver model {model}."
                )
                })?;
                validate_firmware_package_for_model(package_file, &model)?;
                let (target_file, target_url) =
                    firmware_target_for_exact_package(&request.firmware_version, package_file)?;
                (target_file, target_url, None)
            } else {
                let (target_file, target_url) = firmware_target_for_model(
                    &mode_config,
                    &request.firmware_version,
                    &request.firmware_suffix,
                    &model,
                );
                (target_file, target_url, None)
            };
        receivers.push(FirmwarePlanReceiver {
            credential,
            model,
            target_file,
            target_url,
            local_package_path,
        });
    }

    receivers.sort_by_key(|receiver| ip_sort_key(&receiver.credential.addr));
    Ok((mode_config, group, receivers))
}

fn preflight_firmware_receiver(
    client: &Client,
    rx: &FirmwarePlanReceiver,
) -> FirmwarePreflightReceiver {
    let credential = &rx.credential;
    let sys_text = match get_xml_text(
        client,
        credential,
        &format!("http://{}/xml/dynamic/merge.xml?sysData=", credential.addr),
    ) {
        Ok(text) => text,
        Err(err) => {
            return FirmwarePreflightReceiver {
                ip: credential.addr.clone(),
                model: rx.model.clone(),
                status: "Failed".to_string(),
                message: err,
                receiver_name: None,
                firmware_version: None,
                uptime_seconds: None,
                target_file: rx.target_file.clone(),
                target_url: rx.target_url.clone(),
                can_update: false,
            }
        }
    };

    let sys_doc = match roxmltree::Document::parse(&sys_text) {
        Ok(doc) => doc,
        Err(err) => {
            return FirmwarePreflightReceiver {
                ip: credential.addr.clone(),
                model: rx.model.clone(),
                status: "Failed".to_string(),
                message: format!("Invalid receiver sysData XML: {err}"),
                receiver_name: None,
                firmware_version: None,
                uptime_seconds: None,
                target_file: rx.target_file.clone(),
                target_url: rx.target_url.clone(),
                can_update: false,
            }
        }
    };

    let receiver_name = select_text(&sys_doc, "RXName").map(normalize_receiver_name);
    let firmware_version = select_text(&sys_doc, "FWVersion");
    let power_text = get_xml_text(
        client,
        credential,
        &format!("http://{}/xml/dynamic/merge.xml?power=", credential.addr),
    )
    .ok();
    let uptime_seconds = power_text
        .as_deref()
        .and_then(|text| roxmltree::Document::parse(text).ok())
        .and_then(|doc| get_uptime_seconds(&doc));

    if is_receiver_installing_firmware(client, credential).unwrap_or(false) {
        return FirmwarePreflightReceiver {
            ip: credential.addr.clone(),
            model: rx.model.clone(),
            status: "Warning".to_string(),
            message: "Receiver appears to be installing firmware already.".to_string(),
            receiver_name,
            firmware_version,
            uptime_seconds,
            target_file: rx.target_file.clone(),
            target_url: rx.target_url.clone(),
            can_update: false,
        };
    }

    FirmwarePreflightReceiver {
        ip: credential.addr.clone(),
        model: rx.model.clone(),
        status: "Ready".to_string(),
        message: format!("Receiver verified. Target package: {}", rx.target_file),
        receiver_name,
        firmware_version,
        uptime_seconds,
        target_file: rx.target_file.clone(),
        target_url: rx.target_url.clone(),
        can_update: true,
    }
}

fn is_receiver_installing_firmware(
    client: &Client,
    rx: &ReceiverCredential,
) -> Result<bool, String> {
    is_receiver_installing_firmware_with_password(client, rx, &rx.pw)
}

fn is_receiver_installing_firmware_with_password(
    client: &Client,
    rx: &ReceiverCredential,
    password: &str,
) -> Result<bool, String> {
    let response = client
        .get(format!("http://{}/index.html", rx.addr))
        .basic_auth(&rx.user, Some(password))
        .send()
        .map_err(|err| {
            format!(
                "Cannot check install state for {}: {}",
                rx.addr,
                clean_error(err)
            )
        })?;

    if !response.status().is_success() {
        return Err(format!(
            "HTTP {} for {}",
            response.status().as_u16(),
            rx.addr
        ));
    }

    let text = response.text().map_err(|err| {
        format!(
            "Cannot read install state from {}: {}",
            rx.addr,
            clean_error(err)
        )
    })?;
    let lower = text.to_lowercase();
    Ok(INSTALL_MARKERS
        .iter()
        .any(|marker| lower.contains(&marker.to_lowercase())))
}

fn firmware_temp_password() -> Result<String, String> {
    Ok(FIRMWARE_TEMP_PASSWORD.to_string())
}

fn application_admin_password() -> Result<String, String> {
    Ok(APPLICATION_ADMIN_PASSWORD.to_string())
}

fn validate_application_password(candidate: &str) -> Result<(), String> {
    if candidate == application_admin_password()? {
        Ok(())
    } else {
        Err("Invalid application password.".to_string())
    }
}

fn check_initial_firmware_password(
    single_jobs: &Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    job_id: &str,
    rx: &ReceiverCredential,
) -> Result<receiver_password::FirmwareSecurityMode, String> {
    update_single_job_step(single_jobs, job_id, "CheckPassword", "Running", "Running",
        "Checking receiver security mode. Protected receivers use one login check; unprotected receivers do not receive login details.");
    match receiver_password::check_firmware_access(rx, &rx.pw) {
        Ok(receiver_password::FirmwareSecurityMode::PasswordProtected) => {
            update_single_job_step(single_jobs, job_id, "CheckPassword", "Completed", "Running",
                "Receiver login accepted the configured password. No password change has been sent.");
            Ok(receiver_password::FirmwareSecurityMode::PasswordProtected)
        }
        Ok(receiver_password::FirmwareSecurityMode::Unprotected) => {
            update_single_job_step(single_jobs, job_id, "CheckPassword", "Completed", "Running",
                "Receiver security is disabled. Password login, temporary password change, and password recovery will be skipped for this update.");
            Ok(receiver_password::FirmwareSecurityMode::Unprotected)
        }
        Err(error) => {
            let message = format!("Update stopped before the password change for {}. No password change or firmware upload was sent by this job. {error}", rx.addr);
            update_single_job_step(
                single_jobs,
                job_id,
                "CheckPassword",
                "Failed",
                "Running",
                &message,
            );
            Err(message)
        }
    }
}

fn change_receiver_password(
    rx: &ReceiverCredential,
    current_password: &str,
    new_password: &str,
) -> Result<(), String> {
    receiver_password::change(rx, current_password, new_password)
}

fn activate_temporary_firmware_password(
    rx: &ReceiverCredential,
    temporary_password: &str,
) -> Result<bool, String> {
    if rx.pw == temporary_password {
        return Ok(false);
    }
    change_receiver_password(rx, &rx.pw, temporary_password)?;
    verify_receiver_password(rx, temporary_password).map_err(|error| format!(
        "The temporary password change was acknowledged, but its login check failed. Password recovery is required. {error}"
    ))?;
    Ok(true)
}

fn restore_receiver_password_with_retry(
    rx: &ReceiverCredential,
    temporary_password: &str,
) -> Result<(), String> {
    receiver_password::restore(rx, temporary_password)
}

fn upload_local_firmware_package(
    rx: &ReceiverCredential,
    file_path: &Path,
    file_name: &str,
    password: &str,
) -> Result<(), String> {
    let file_bytes = fs::read(file_path).map_err(|err| {
        format!(
            "Cannot read local firmware package {}: {err}",
            file_path.display()
        )
    })?;
    let boundary = format!("qa-receiver-center-{}", now_millis());
    let mut body = Vec::with_capacity(file_bytes.len() + 256);
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"firmware\"; filename=\"{file_name}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(&file_bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let client = Client::builder()
        .timeout(Duration::from_secs(3600))
        .connect_timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| format!("HTTP client error: {err}"))?;
    let response = client
        .post(format!(
            "http://{}/cgi-bin/firmware_upload.xml?failsafe=1&seqNum=1&forceMonUp=1",
            rx.addr
        ))
        .basic_auth(&rx.user, Some(password))
        .header(EXPECT, "")
        .header(
            CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(body)
        .send();

    match response {
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                Ok(())
            } else {
                Err(format!(
                    "Firmware upload to {} failed with HTTP {}.",
                    rx.addr,
                    status.as_u16()
                ))
            }
        }
        Err(err) if err.is_timeout() || err.is_connect() => Ok(()),
        Err(err) => Err(format!(
            "Firmware upload to {} failed: {}",
            rx.addr,
            clean_error(err)
        )),
    }
}

fn create_receiver_clone_backup(
    client: &Client,
    rx: &ReceiverCredential,
    job_id: &str,
) -> Result<CloneFileBackup, String> {
    let sys_text = get_xml_text(
        client,
        rx,
        &format!("http://{}/xml/dynamic/merge.xml?sysData=", rx.addr),
    )?;
    let sys_doc = roxmltree::Document::parse(&sys_text)
        .map_err(|err| format!("Invalid sysData XML from {}: {err}", rx.addr))?;
    let cache_dir = select_text(&sys_doc, "cachedir");

    let clone_base_name = build_clone_file_base_name(&rx.addr);
    let clone_xml_name = format!("{clone_base_name}.xml");
    let clone_selector_name = format!("IP{}.xml", receiver_ip_tail(&rx.addr));
    let clone_download_candidates =
        build_clone_file_download_candidates(&clone_xml_name, &clone_selector_name);

    let create_url = format!(
        "http://{}/cgi-bin/app_fileUpdate.xml?operation=8&fileNumber=1&cloneFileName={}&Year=2026&Month=1&Day=1&Hour=0&Minute=0&RepeatMin=0&newAppFileName=&newCloneFileName={}&cloneSecurityEnable=on&cloneTcpUdpPortEnable=on&cloneEtherBootEnable=on&cloneHttpEnable=on&cloneEmailFtpNtpEnable=on&cloneDataLoggerEnable=on&clonePositionEnable=on&cloneAlmEnable=on&cloneInsEnable=on&cloneMiscellaneousEnable=on&cloneAllAppfilesEnable=on&missingPasswordDefault=",
        rx.addr, clone_selector_name, clone_base_name
    );
    let create_response = client
        .get(&create_url)
        .basic_auth(&rx.user, Some(&rx.pw))
        .send()
        .map_err(|err| {
            format!(
                "Cannot create clone file for {}: {}",
                rx.addr,
                clean_error(err)
            )
        })?;
    let create_status = create_response.status();
    if !create_status.is_success() {
        return Err(format!(
            "Cannot create clone file for {}: HTTP {}.",
            rx.addr,
            create_status.as_u16()
        ));
    }
    let create_text = create_response.text().unwrap_or_default();
    if is_clone_file_failure_response(&create_text) {
        return Err(format!(
            "Receiver {} rejected clone file creation request.",
            rx.addr
        ));
    }

    let (downloaded_file_name, clone_text) =
        download_receiver_clone_file(client, rx, cache_dir.as_deref(), &clone_download_candidates)?;
    let local_path = save_clone_file_backup(job_id, &rx.addr, &downloaded_file_name, &clone_text)?;

    Ok(CloneFileBackup {
        file_name: downloaded_file_name,
        local_path,
    })
}

fn build_clone_file_base_name(ip: &str) -> String {
    format!(
        "{}_IP{}",
        today_clone_date(),
        sanitize_clone_name_part(receiver_ip_tail(ip))
    )
}

fn build_clone_file_download_candidates(
    requested_xml_name: &str,
    selector_xml_name: &str,
) -> Vec<String> {
    let mut candidates = Vec::new();
    push_unique_candidate(&mut candidates, requested_xml_name.to_string());
    push_unique_candidate(&mut candidates, selector_xml_name.to_string());
    candidates
}

fn push_unique_candidate(candidates: &mut Vec<String>, candidate: String) {
    if !candidate.trim().is_empty() && !candidates.iter().any(|item| item == &candidate) {
        candidates.push(candidate);
    }
}

fn receiver_ip_tail(ip: &str) -> &str {
    ip.rsplit('.')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or(ip)
}

fn download_receiver_clone_file(
    client: &Client,
    rx: &ReceiverCredential,
    cache_dir: Option<&str>,
    clone_file_names: &[String],
) -> Result<(String, String), String> {
    let urls = build_clone_file_download_urls(&rx.addr, cache_dir, clone_file_names);

    let mut last_error = String::new();
    let mut stable_candidate: Option<(String, String, usize, u8)> = None;
    for attempt in 0..40 {
        for (file_name, url) in &urls {
            match get_xml_text(client, rx, url) {
                Ok(text) if is_valid_clone_file_response(&text) => {
                    let length = text.len();
                    match &stable_candidate {
                        Some((stable_name, stable_text, stable_length, stable_count))
                            if stable_name == file_name
                                && *stable_length == length
                                && stable_text == &text =>
                        {
                            let next_count = stable_count + 1;
                            if next_count >= 2 {
                                return Ok((file_name.clone(), text));
                            }
                            stable_candidate = Some((file_name.clone(), text, length, next_count));
                        }
                        _ => {
                            stable_candidate = Some((file_name.clone(), text, length, 1));
                        }
                    }
                }
                Ok(_) => {
                    last_error = format!(
                        "Receiver returned an empty, incomplete or invalid clone file while reading {file_name}."
                    );
                }
                Err(err) => last_error = err,
            }
        }

        if attempt < 39 {
            thread::sleep(Duration::from_millis(500));
        }
    }

    Err(format!(
        "Cannot download valid clone file from {}. Tried: {}. {}",
        rx.addr,
        clone_file_names.join(", "),
        last_error
    ))
}

fn build_clone_file_download_urls(
    ip: &str,
    cache_dir: Option<&str>,
    clone_file_names: &[String],
) -> Vec<(String, String)> {
    let mut urls = Vec::new();
    for file_name in clone_file_names {
        urls.push((
            file_name.clone(),
            format!("http://{}/clone_file/{}?gzipFlag=false", ip, file_name),
        ));
        if let Some(cache_dir) = cache_dir {
            urls.push((
                file_name.clone(),
                format!(
                    "http://{}/{}/clone_file/{}?gzipFlag=false",
                    ip, cache_dir, file_name
                ),
            ));
        }
    }
    urls
}

fn is_clone_file_failure_response(text: &str) -> bool {
    let normalized = text.trim().to_ascii_lowercase();
    normalized == "<fail>1</fail>" || normalized.contains("<fail>1</fail>")
}

fn is_valid_clone_file_response(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty()
        && !is_clone_file_failure_response(trimmed)
        && trimmed.starts_with('<')
        && trimmed.contains("</CloneConfigData>")
}

fn save_clone_file_backup(
    job_id: &str,
    ip: &str,
    file_name: &str,
    contents: &str,
) -> Result<String, String> {
    if !is_valid_clone_file_response(contents) {
        return Err(format!(
            "Cannot save clone file backup {} because receiver returned empty or invalid XML.",
            file_name
        ));
    }

    let base = qa_receiver_center_data_dir()?.join("clone-files");
    let safe_ip = sanitize_clone_name_part(ip);
    let dir = base.join(safe_ip).join(job_id);
    fs::create_dir_all(&dir).map_err(|err| {
        format!(
            "Cannot create clone file backup directory {}: {err}",
            dir.display()
        )
    })?;
    let path = dir.join(file_name);
    fs::write(&path, contents)
        .map_err(|err| format!("Cannot write clone file backup {}: {err}", path.display()))?;
    Ok(path.display().to_string())
}

fn sanitize_clone_name_part(value: &str) -> String {
    let mut output = String::new();
    let mut last_was_separator = false;
    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch);
            last_was_separator = false;
        } else if !last_was_separator {
            output.push('_');
            last_was_separator = true;
        }
    }

    let output = output.trim_matches('_').to_string();
    if output.is_empty() {
        "Unknown".to_string()
    } else {
        output
    }
}

fn next_available_download_path(file_name: &str) -> Result<PathBuf, String> {
    let downloads = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .map(|path| path.join("Downloads"))
        .ok_or_else(|| "Cannot resolve user Downloads directory.".to_string())?;
    fs::create_dir_all(&downloads).map_err(|err| {
        format!(
            "Cannot create Downloads directory {}: {err}",
            downloads.display()
        )
    })?;

    let requested = downloads.join(file_name);
    if !requested.exists() {
        return Ok(requested);
    }

    let source_name = Path::new(file_name);
    let stem = source_name
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("clone_file");
    let extension = source_name.extension().and_then(|value| value.to_str());
    for index in 1..1000 {
        let candidate_name = if let Some(extension) = extension {
            format!("{stem} ({index}).{extension}")
        } else {
            format!("{stem} ({index})")
        };
        let candidate = downloads.join(candidate_name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err("Cannot create a unique clone file name in Downloads.".to_string())
}

fn wait_for_receiver_install_live(
    rx: &ReceiverCredential,
    password: &str,
    single_jobs: &Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    control: &FirmwareJobControl,
    job_id: &str,
    initial_firmware_version: Option<&str>,
    initial_uptime_seconds: Option<i64>,
) -> Result<(), String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(4))
        .build()
        .map_err(|err| format!("HTTP client error: {err}"))?;
    let started_at = Instant::now();
    let timeout = Duration::from_secs(30 * 60);
    let interval = Duration::from_secs(2);
    let uses_fast_image_progress = single_job_uses_fast_image(single_jobs, job_id);
    // A completed .img upload can reboot too quickly for the first XML progress probe.
    // Treat the accepted upload itself as activity so offline/uptime signals can finish the job.
    let mut saw_update_activity =
        uses_fast_image_progress || current_single_job_stage(single_jobs, job_id).is_some();
    let mut saw_cleanup = false;
    let mut saw_reboot_offline = false;
    let mut stable_completion_probes = 0usize;
    let mut offline_count = 0usize;
    let mut last_probe_message = String::from("Waiting for receiver firmware status.");

    while started_at.elapsed() < timeout {
        match receiver_firmware_live_probe(&client, rx, password) {
            Ok(probe) => {
                offline_count = 0;
                let update_mode = probe.update_mode.clone();
                let update_stage = probe.update_stage;
                let update_in_progress = probe.update_in_progress;
                let firmware_status = probe.firmware_status.clone();
                let fw_status = probe.fw_status.clone();
                let observed_firmware_version = probe.firmware_version.clone();
                let observed_uptime_seconds = probe.uptime_seconds;
                update_single_job_probe(single_jobs, job_id, probe);
                let effective_stage =
                    current_single_job_stage(single_jobs, job_id).or(update_stage);
                if matches!(effective_stage, Some(3)) {
                    saw_cleanup = true;
                }

                let status_text = best_firmware_status_text(
                    update_mode.as_deref(),
                    fw_status.as_deref(),
                    firmware_status.as_deref(),
                );
                if !status_text.trim().is_empty() {
                    last_probe_message = status_text;
                    update_single_job_message(
                        single_jobs,
                        job_id,
                        "MonitorInstall",
                        "Running",
                        "Running",
                        &last_probe_message,
                    );
                    if update_in_progress || firmware_status_indicates_activity(&last_probe_message)
                    {
                        saw_update_activity = true;
                    }
                }

                let interface_reports_installing =
                    match is_receiver_installing_firmware_with_password(&client, rx, password) {
                        Ok(true) => {
                            saw_update_activity = true;
                            update_single_job_message(
                                single_jobs,
                                job_id,
                                "MonitorInstall",
                                "Running",
                                "Running",
                                "Receiver web interface reports firmware installation in progress.",
                            );
                            true
                        }
                        Ok(false) => false,
                        Err(error) if is_receiver_authentication_error(&error) => {
                            return Err(error)
                        }
                        Err(_) => {
                            saw_update_activity = true;
                            false
                        }
                    };

                if control.is_cancel_requested()
                    && !update_in_progress
                    && !interface_reports_installing
                {
                    let message = "Cancellation confirmed after the receiver reported an idle firmware state. Preparing password recovery.";
                    update_single_job_message(
                        single_jobs,
                        job_id,
                        "Cancelling",
                        "Running",
                        "Running",
                        message,
                    );
                    return Err(message.to_string());
                }

                let firmware_changed = firmware_release_changed(
                    initial_firmware_version,
                    observed_firmware_version.as_deref(),
                );
                let receiver_rebooted =
                    receiver_uptime_was_reset(initial_uptime_seconds, observed_uptime_seconds);
                if uses_fast_image_progress
                    && !update_in_progress
                    && (saw_reboot_offline || receiver_rebooted)
                {
                    update_single_job_message(
                        single_jobs,
                        job_id,
                        "ReceiverOnline",
                        "Running",
                        "Running",
                        "Receiver is online after restart. Confirming a stable connection before password recovery.",
                    );
                }
                let normal_install_completed = saw_cleanup && saw_reboot_offline;
                let fast_install_completed = saw_update_activity
                    && (receiver_rebooted
                        || (firmware_changed && started_at.elapsed() >= Duration::from_secs(8)));

                if !update_in_progress && (normal_install_completed || fast_install_completed) {
                    stable_completion_probes += 1;
                } else {
                    stable_completion_probes = 0;
                }

                if stable_completion_probes >= 2 {
                    update_single_job_message(
                        single_jobs,
                        job_id,
                        if uses_fast_image_progress {
                            "ReceiverOnline"
                        } else {
                            "MonitorInstall"
                        },
                        "Running",
                        "Running",
                        if normal_install_completed {
                            "Receiver completed the firmware stages and returned online. Preparing password recovery."
                        } else {
                            "Receiver returned online with the updated firmware and rebooted uptime. Preparing password recovery."
                        },
                    );
                    return Ok(());
                }
            }
            Err(err) => {
                if is_receiver_authentication_error(&err) {
                    return Err(format!(
                        "Firmware monitoring stopped to prevent repeated rejected logins. {err}"
                    ));
                }
                offline_count += 1;
                if saw_update_activity || saw_cleanup {
                    saw_reboot_offline = true;
                }
                let fast_image_is_restarting =
                    uses_fast_image_progress && (saw_update_activity || saw_cleanup);
                let restart_message = if fast_image_is_restarting {
                    format!(
                        "Firmware programming finished. Waiting for the receiver to return after restart ({offline_count})."
                    )
                } else {
                    format!(
                        "Receiver is not responding during install/reboot window ({offline_count}): {err}"
                    )
                };
                update_single_job_message(
                    single_jobs,
                    job_id,
                    if fast_image_is_restarting {
                        "ReceiverRestart"
                    } else {
                        "MonitorInstall"
                    },
                    "Running",
                    "Running",
                    &restart_message,
                );
            }
        }

        thread::sleep(interval);
    }

    Err(format!(
        "Timed out waiting for {} to finish firmware install. Last status: {}",
        rx.addr, last_probe_message
    ))
}

fn firmware_release_version(value: &str) -> Option<(u32, u32)> {
    let dotted = Regex::new(r"(?i)(\d{1,3})\.(\d{1,2})").ok()?;
    if let Some(captures) = dotted.captures(value) {
        return Some((
            captures.get(1)?.as_str().parse().ok()?,
            captures.get(2)?.as_str().parse().ok()?,
        ));
    }

    let package = Regex::new(r"(?i)(?:^|_)V(\d{3,4})(?:_|\.|$)").ok()?;
    let digits = package.captures(value)?.get(1)?.as_str();
    let split_at = digits.len().checked_sub(2)?;
    Some((
        digits[..split_at].parse().ok()?,
        digits[split_at..].parse().ok()?,
    ))
}

fn is_firmware_downgrade(current_version: Option<&str>, target_file: &str) -> bool {
    let Some(current) = current_version.and_then(firmware_release_version) else {
        return false;
    };
    let Some(target) = firmware_release_version(target_file) else {
        return false;
    };
    target < current
}

fn firmware_release_changed(initial: Option<&str>, observed: Option<&str>) -> bool {
    match (
        initial.and_then(firmware_release_version),
        observed.and_then(firmware_release_version),
    ) {
        (Some(before), Some(after)) => before != after,
        _ => false,
    }
}

fn receiver_uptime_was_reset(initial: Option<i64>, observed: Option<i64>) -> bool {
    matches!((initial, observed), (Some(before), Some(after)) if after.saturating_add(30) < before)
}

fn monitor_receiver_firmware_status_until_stop(
    rx: &ReceiverCredential,
    password: &str,
    single_jobs: &Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    job_id: &str,
    phase: &str,
    stop: Arc<AtomicBool>,
) {
    let client = match Client::builder()
        .timeout(Duration::from_secs(3))
        .connect_timeout(Duration::from_secs(1))
        .build()
    {
        Ok(client) => client,
        Err(_) => return,
    };

    while !stop.load(Ordering::SeqCst) {
        let probe = receiver_firmware_live_probe(&client, rx, password);
        if probe
            .as_ref()
            .err()
            .is_some_and(|error| is_receiver_authentication_error(error))
        {
            update_single_job_message(single_jobs, job_id, phase, "Running", "Running",
                "Firmware monitoring stopped after a rejected login. Password recovery will follow the upload request.");
            break;
        }
        if let Ok(probe) = probe {
            let update_mode = probe.update_mode.clone();
            let firmware_status = probe.firmware_status.clone();
            let fw_status = probe.fw_status.clone();
            update_single_job_probe(single_jobs, job_id, probe);

            let status_text = best_firmware_status_text(
                update_mode.as_deref(),
                fw_status.as_deref(),
                firmware_status.as_deref(),
            );

            if !status_text.trim().is_empty() {
                update_single_job_message(
                    single_jobs,
                    job_id,
                    phase,
                    "Running",
                    "Running",
                    &status_text,
                );
            }
        }

        thread::sleep(Duration::from_secs(1));
    }
}

fn receiver_firmware_live_probe(
    client: &Client,
    rx: &ReceiverCredential,
    password: &str,
) -> Result<FirmwareReceiverLiveProbe, String> {
    let status_url = format!(
        "http://{}/xml/dynamic/merge.xml?fw_status=&fw_check=&sysData=",
        rx.addr
    );
    let status_text = get_xml_text_with_password(client, rx, password, &status_url)?;
    let status_doc = roxmltree::Document::parse(&status_text)
        .map_err(|err| format!("Invalid firmware status XML from {}: {err}", rx.addr))?;
    let fw_status_node = find_node(&status_doc, "fw_status")
        .or_else(|| find_node(&status_doc, "fwStatus"))
        .or_else(|| find_node(&status_doc, "FWStatus"));
    let firmware_status_node = find_node(&status_doc, "firmware_status")
        .or_else(|| find_node(&status_doc, "firmwareStatus"))
        .or_else(|| find_node(&status_doc, "FirmwareStatus"));

    let raw_mode = fw_status_node
        .and_then(|node| select_nested_text_from_node(node, &["status", "mode"]))
        .or_else(|| fw_status_node.and_then(|node| select_text_from_node(node, "mode")));
    let stage_percent = fw_status_node
        .and_then(|node| select_nested_text_from_node(node, &["status", "percent_done"]))
        .or_else(|| fw_status_node.and_then(|node| select_text_from_node(node, "percent_done")))
        .and_then(|value| parse_percent(&value));
    let stage = raw_mode
        .as_deref()
        .and_then(firmware_install_stage_from_mode);
    let update_mode = stage
        .as_ref()
        .map(|stage| stage.label.to_string())
        .or_else(|| raw_mode.clone());
    let update_stage = stage.as_ref().map(|stage| stage.index);
    let update_in_progress = fw_status_node
        .and_then(|node| select_text_from_node(node, "in_progress"))
        .map(|value| value.trim() == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let active_checksum =
        fw_status_node.and_then(|node| select_text_from_node(node, "active_checksum"));
    let fw_status = render_receiver_fw_status(update_mode.as_deref(), None, update_in_progress)
        .or_else(|| fw_status_node.and_then(render_node_text));
    let firmware_status = firmware_status_node.and_then(render_node_text);
    let power_url = format!("http://{}/xml/dynamic/merge.xml?power=", rx.addr);
    let power = get_xml_text_with_password(client, rx, password, &power_url);
    if let Err(error) = &power {
        if is_receiver_authentication_error(error) {
            return Err(error.clone());
        }
    }
    let uptime_seconds = power.ok().and_then(|text| {
        roxmltree::Document::parse(&text)
            .ok()
            .and_then(|doc| get_uptime_seconds(&doc))
    });

    Ok(FirmwareReceiverLiveProbe {
        firmware_status,
        fw_status,
        update_mode,
        update_stage,
        update_stage_percent: stage_percent,
        update_in_progress,
        active_checksum,
        receiver_name: select_text(&status_doc, "RXName").map(normalize_receiver_name),
        firmware_version: select_text(&status_doc, "FWVersion"),
        uptime_seconds,
    })
}

fn firmware_status_indicates_activity(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "install", "upload", "update", "reboot", "program", "write", "erase", "failsafe",
        "progress", "busy",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn firmware_install_stage_from_mode(mode: &str) -> Option<FirmwareInstallStage> {
    let normalized = mode.trim().to_ascii_uppercase().replace([' ', '-'], "_");

    if normalized.contains("CLEAN")
        || normalized.contains("CLEANUP")
        || normalized.contains("PROGRAMMING_AREA")
    {
        return Some(FirmwareInstallStage {
            index: 3,
            label: "Cleaning up failsafe programming area",
        });
    }

    if normalized.contains("FAILSAFE_REGION")
        || normalized.contains("FROM_FAILSAFE")
        || normalized.contains("INSTALL_FAILSAFE")
    {
        return Some(FirmwareInstallStage {
            index: 2,
            label: "Installing firmware from failsafe region",
        });
    }

    if normalized.contains("ACTIVE_FAILSAFE_COPY")
        || normalized.contains("FAILSAFE_COPY")
        || normalized.contains("BROWSER_UPLOAD")
    {
        return Some(FirmwareInstallStage {
            index: 1,
            label: "Installing failsafe copy of firmware",
        });
    }

    None
}

fn firmware_overall_percent(stage: u8, stage_percent: f64) -> f64 {
    let stage_index = stage.clamp(1, 3) as f64 - 1.0;
    let safe_stage_percent = stage_percent.clamp(0.0, 100.0);
    ((stage_index * 100.0) + safe_stage_percent) / 3.0
}

fn is_fast_firmware_image(file_name: &str) -> bool {
    Path::new(file_name)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("img"))
}

fn fast_image_install_percent(receiver_percent: f64) -> f64 {
    let normalized = receiver_percent.clamp(0.0, 100.0) / 100.0;
    FAST_IMAGE_INSTALL_START_PROGRESS
        + normalized * (FAST_IMAGE_INSTALL_COMPLETE_PROGRESS - FAST_IMAGE_INSTALL_START_PROGRESS)
}

fn fast_image_workflow_progress(phase: &str, step_status: &str) -> Option<f64> {
    match (phase, step_status) {
        ("DownloadPackage", "Running") => Some(1.0),
        ("DownloadPackage", "Completed") => Some(3.0),
        ("Preflight", "Running") => Some(2.0),
        ("Preflight", "Completed") => Some(4.0),
        ("UpdateSafety", "Running") => Some(4.0),
        ("UpdateSafety", "Completed") => Some(5.0),
        ("CloneConfig", "Running") => Some(6.0),
        ("CloneConfig", "Completed") => Some(8.0),
        ("DowngradeApproval", "Running") => Some(8.0),
        ("DowngradeApproval", "Completed") => Some(9.0),
        ("ChangePassword", "Running") => Some(10.0),
        ("ChangePassword", "Completed") => Some(12.0),
        ("UploadFirmware", "Running") => Some(FAST_IMAGE_INSTALL_START_PROGRESS),
        ("UploadFirmware", "Completed") => Some(FAST_IMAGE_INSTALL_COMPLETE_PROGRESS),
        ("MonitorInstall", "Running") => Some(74.0),
        ("MonitorInstall", "Completed") => Some(FAST_IMAGE_ONLINE_PROGRESS),
        ("ReceiverRestart", "Running") => Some(FAST_IMAGE_RESTART_PROGRESS),
        ("ReceiverOnline", "Running") => Some(FAST_IMAGE_ONLINE_PROGRESS),
        ("RestorePassword", "Running") => Some(92.0),
        ("RestorePassword", "Completed") => Some(FAST_IMAGE_PASSWORD_RESTORED_PROGRESS),
        ("VerifyResult", "Running") => Some(FAST_IMAGE_VERIFY_PROGRESS),
        ("VerifyResult", "Completed") => Some(99.0),
        _ => None,
    }
}

fn apply_fast_image_progress_floor(
    job: &mut FirmwareSingleUpdateJobStatus,
    phase: &str,
    step_status: &str,
) {
    if !is_fast_firmware_image(&job.target_file) {
        return;
    }

    if let Some(progress) = fast_image_workflow_progress(phase, step_status) {
        job.update_percent = Some(
            job.update_percent
                .map(|current| current.max(progress))
                .unwrap_or(progress),
        );
    }
}

fn best_firmware_status_text(
    update_mode: Option<&str>,
    fw_status: Option<&str>,
    firmware_status: Option<&str>,
) -> String {
    [update_mode, fw_status, firmware_status]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|value| !value.is_empty())
        .unwrap_or("")
        .to_string()
}

fn resolve_firmware_update_stage(
    current_stage: Option<u8>,
    previous_stage_percent: Option<f64>,
    probe: &FirmwareReceiverLiveProbe,
) -> Option<u8> {
    if let Some(mode_stage) = probe.update_stage {
        return Some(
            current_stage
                .map_or(mode_stage, |current| current.max(mode_stage))
                .clamp(1, 3),
        );
    }

    let current_percent = probe.update_stage_percent?;
    let current_stage = current_stage.unwrap_or(1).clamp(1, 3);
    let Some(previous_percent) = previous_stage_percent else {
        return Some(current_stage);
    };

    if current_percent + 18.0 < previous_percent {
        Some((current_stage + 1).min(3))
    } else {
        Some(current_stage)
    }
}

fn current_single_job_stage(
    single_jobs: &Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    job_id: &str,
) -> Option<u8> {
    single_jobs
        .lock()
        .ok()
        .and_then(|jobs| jobs.get(job_id).and_then(|job| job.update_stage))
}

fn single_job_uses_fast_image(
    single_jobs: &Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    job_id: &str,
) -> bool {
    single_jobs
        .lock()
        .ok()
        .and_then(|jobs| {
            jobs.get(job_id)
                .map(|job| is_fast_firmware_image(&job.target_file))
        })
        .unwrap_or(false)
}

fn active_job_status(status: &str) -> &str {
    match status {
        "Completed" | "Failed" => "Running",
        "Cancelled" => "Cancelling",
        _ => status,
    }
}

fn password_recovery_error(job: &FirmwareSingleUpdateJobStatus) -> Option<String> {
    let restore = job
        .steps
        .iter()
        .rev()
        .find(|step| step.state == "RestorePassword");
    if let Some(step) = restore {
        if step.status == "Completed" {
            return None;
        }
        if step.status == "Failed" {
            return Some(step.message.clone());
        }
    }
    if job
        .steps
        .iter()
        .any(|step| step.state == "ChangePassword" && step.status != "Skipped")
    {
        return Some(format!(
            "PASSWORD RECOVERY REQUIRED for {}. The configured password has not been verified. Stop other scripts using this receiver and check its password manually.",
            job.ip
        ));
    }
    None
}

fn update_single_job_step(
    single_jobs: &Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    job_id: &str,
    phase: &str,
    step_status: &str,
    job_status: &str,
    message: &str,
) {
    if let Ok(mut jobs) = single_jobs.lock() {
        if let Some(job) = jobs.get_mut(job_id) {
            job.phase = phase.to_string();
            // Recovery may still be running after a failed step. Only completion
            // may publish a terminal status, otherwise the UI stops polling early.
            job.status = active_job_status(job_status).to_string();
            job.message = message.to_string();
            push_workflow_step(&mut job.steps, phase, step_status, message);
            apply_fast_image_progress_floor(job, phase, step_status);
        }
    }
}

fn update_single_job_message(
    single_jobs: &Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    job_id: &str,
    phase: &str,
    step_status: &str,
    job_status: &str,
    message: &str,
) {
    if let Ok(mut jobs) = single_jobs.lock() {
        if let Some(job) = jobs.get_mut(job_id) {
            job.phase = phase.to_string();
            job.status = active_job_status(job_status).to_string();
            job.message = message.to_string();
            if let Some(step) = job.steps.iter_mut().rev().find(|step| step.state == phase) {
                step.status = step_status.to_string();
                step.message = message.to_string();
            } else {
                push_workflow_step(&mut job.steps, phase, step_status, message);
            }
            apply_fast_image_progress_floor(job, phase, step_status);
        }
    }
}

fn update_single_job_metadata(
    single_jobs: &Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    job_id: &str,
    receiver_name: Option<String>,
    firmware_version: Option<String>,
    uptime_seconds: Option<i64>,
) {
    if let Ok(mut jobs) = single_jobs.lock() {
        if let Some(job) = jobs.get_mut(job_id) {
            if receiver_name.is_some() {
                job.receiver_name = receiver_name;
            }
            if firmware_version.is_some() {
                job.firmware_version = firmware_version;
            }
            if uptime_seconds.is_some() {
                job.uptime_seconds = uptime_seconds;
            }
            job.last_seen_at = Some(now_seconds());
        }
    }
}

fn update_single_job_clone_file(
    single_jobs: &Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    job_id: &str,
    clone_file: &CloneFileBackup,
) {
    if let Ok(mut jobs) = single_jobs.lock() {
        if let Some(job) = jobs.get_mut(job_id) {
            job.clone_file_name = Some(clone_file.file_name.clone());
            job.clone_file_path = Some(clone_file.local_path.clone());
        }
    }
}

fn update_single_job_probe(
    single_jobs: &Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    job_id: &str,
    probe: FirmwareReceiverLiveProbe,
) {
    if let Ok(mut jobs) = single_jobs.lock() {
        if let Some(job) = jobs.get_mut(job_id) {
            let resolved_stage =
                resolve_firmware_update_stage(job.update_stage, job.update_stage_percent, &probe);
            if probe.receiver_name.is_some() {
                job.receiver_name = probe.receiver_name;
            }
            if probe.firmware_version.is_some() {
                job.firmware_version = probe.firmware_version;
            }
            if probe.uptime_seconds.is_some() {
                job.uptime_seconds = probe.uptime_seconds;
            }
            if probe.firmware_status.is_some() {
                job.firmware_status = probe.firmware_status;
            }
            if probe.fw_status.is_some() {
                job.fw_status = probe.fw_status;
            }
            if probe.update_mode.is_some() {
                job.update_mode = probe.update_mode;
            }
            if let Some(stage) = resolved_stage {
                job.update_stage = Some(stage);
            }
            if let (Some(stage), Some(stage_percent)) =
                (job.update_stage, probe.update_stage_percent)
            {
                let next_percent = if is_fast_firmware_image(&job.target_file) {
                    fast_image_install_percent(stage_percent)
                } else {
                    firmware_overall_percent(stage, stage_percent).min(99.0)
                };
                job.update_percent = Some(
                    job.update_percent
                        .map(|current| current.max(next_percent))
                        .unwrap_or(next_percent),
                );
                job.update_stage_percent = Some(stage_percent);
            }
            job.update_in_progress = probe.update_in_progress;
            if probe.active_checksum.is_some() {
                job.active_checksum = probe.active_checksum;
            }
            job.last_seen_at = Some(now_seconds());
        }
    }
}

fn finish_single_job(job: &mut FirmwareSingleUpdateJobStatus, status: &str, message: &str) {
    let recovery_error = password_recovery_error(job);
    let status = if recovery_error.is_some() {
        "Failed"
    } else {
        status
    };
    job.status = status.to_string();
    job.message = recovery_error.unwrap_or_else(|| message.to_string());
    if job.status == "Failed" && job.message.starts_with("PASSWORD RECOVERY REQUIRED") {
        job.phase = "RestorePassword".to_string();
    }
    if status == "Completed" {
        job.phase = "Completed".to_string();
        job.update_mode = Some("Update completed".to_string());
        job.update_percent = Some(100.0);
        job.update_stage = None;
        job.update_stage_percent = None;
    } else if status == "Cancelled" {
        job.phase = "Cancelled".to_string();
        job.update_mode = Some("Update cancelled".to_string());
    }
    job.update_in_progress = false;
    job.completed_at = Some(now_seconds());
}

fn complete_single_job(
    single_jobs: &Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
    job_id: &str,
    status: &str,
    message: &str,
) {
    let mut history_entry = None;
    if let Ok(mut jobs) = single_jobs.lock() {
        if let Some(job) = jobs.get_mut(job_id) {
            let already_completed = job.completed_at.is_some();
            finish_single_job(job, status, message);
            if !already_completed {
                history_entry = Some(single_update_history_entry_from_job(job));
            }
        }
    }

    if let Some(entry) = history_entry {
        let _ = append_single_update_history(entry);
    }
}

fn single_update_history_entry_from_job(
    job: &FirmwareSingleUpdateJobStatus,
) -> FirmwareSingleUpdateHistoryEntry {
    let mut errors: Vec<String> = job
        .steps
        .iter()
        .filter(|step| step.status == "Failed" || step.status == "Warning")
        .map(|step| format!("{}: {}", format_workflow_state(&step.state), step.message))
        .collect();

    if job.status == "Failed" && errors.is_empty() {
        errors.push(job.message.clone());
    }

    FirmwareSingleUpdateHistoryEntry {
        job_id: job.job_id.clone(),
        batch_id: job.batch_id.clone(),
        operation_type: job.operation_type.clone(),
        group_name: job.group_name.clone(),
        ip: job.ip.clone(),
        receiver_name: job.receiver_name.clone(),
        model: job.model.clone(),
        target_file: job.target_file.clone(),
        started_at: job.started_at.clone(),
        completed_at: job.completed_at.clone(),
        result: job.status.clone(),
        final_firmware_version: job.firmware_version.clone(),
        clone_file_name: job.clone_file_name.clone(),
        clone_file_path: job.clone_file_path.clone(),
        errors,
    }
}

fn single_update_history_path() -> Result<PathBuf, String> {
    Ok(qa_receiver_center_data_dir()?.join("single-update-history.json"))
}

fn registry_history_cache_path() -> Result<PathBuf, String> {
    Ok(qa_receiver_center_data_dir()?.join("shared-update-history-cache.json"))
}

fn qa_receiver_center_data_dir() -> Result<PathBuf, String> {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from))
        .ok_or_else(|| "Cannot resolve application data directory.".to_string())?;
    Ok(base.join("QA Receiver Center"))
}

fn update_safety_settings_path() -> Result<PathBuf, String> {
    Ok(qa_receiver_center_data_dir()?.join("update-safety-settings.json"))
}

fn validate_update_safety_settings(settings: &UpdateSafetySettings) -> Result<(), String> {
    let cycle = CONVERGENCE_RESTART_CYCLE_MINUTES;
    if settings.post_restart_delay_minutes >= cycle {
        return Err(format!(
            "Delay after restart must be less than {cycle} minutes."
        ));
    }
    if settings.pre_restart_safety_margin_minutes >= cycle {
        return Err(format!(
            "Safety margin before restart must be less than {cycle} minutes."
        ));
    }
    if settings.post_restart_delay_minutes + settings.pre_restart_safety_margin_minutes >= cycle {
        return Err(
            "The restart delay and safety margin must leave a valid update window.".to_string(),
        );
    }
    Ok(())
}

fn registry_setting_url(key: &str) -> String {
    format!("{}/api/v1/settings/{key}", registry_base_url())
}

fn registry_base_url() -> String {
    std::env::var("QA_RECEIVER_REGISTRY_URL")
        .unwrap_or_else(|_| RECEIVER_REGISTRY_BASE_URL.to_string())
        .trim_end_matches('/')
        .to_string()
}

fn registry_api_url(path_segments: &[&str]) -> Result<Url, String> {
    let mut url = Url::parse(&registry_base_url())
        .map_err(|err| format!("Receiver registry URL is invalid: {err}"))?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| "Receiver registry URL cannot contain this base path.".to_string())?;
        segments.pop_if_empty();
        segments.extend(["api", "v1"]);
        segments.extend(path_segments.iter().copied());
    }
    Ok(url)
}

fn registry_ip_admin_call<T: DeserializeOwned>(
    method: Method,
    url: Url,
    body: Option<String>,
    action: &str,
) -> Result<T, String> {
    let mut request = registry_http_client()?
        .request(method, url.clone())
        .header("X-QA-User", registry_actor());
    if let Some(body) = body {
        request = request.header(CONTENT_TYPE, "application/json").body(body);
    }
    let response = request
        .send()
        .map_err(|err| format!("Cannot {action} at {url}: {err}"))?;
    if !response.status().is_success() {
        return Err(registry_response_error(action, url.as_str(), response));
    }
    let response_body = response
        .text()
        .map_err(|err| format!("Cannot read {action} response from {url}: {err}"))?;
    serde_json::from_str(&response_body)
        .map_err(|err| format!("The {action} response from {url} is invalid: {err}"))
}

#[tauri::command]
async fn get_ip_inventory() -> Result<Vec<IpInventoryRecord>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let url = registry_api_url(&["ip-addresses"])?;
        registry_ip_admin_call(Method::GET, url, None, "read IP inventory")
    })
    .await
    .map_err(|err| format!("IP inventory task stopped unexpectedly: {err}"))?
}

#[tauri::command]
async fn update_ip_inventory(
    ip_address: String,
    request: UpdateIpInventoryRequest,
) -> Result<IpInventoryRecord, String> {
    tauri::async_runtime::spawn_blocking(move || {
        ip_address
            .parse::<Ipv4Addr>()
            .map_err(|_| format!("Invalid IP address: {ip_address}"))?;
        let url = registry_api_url(&["ip-addresses", &ip_address])?;
        let body = serde_json::to_string(&request)
            .map_err(|err| format!("Cannot prepare IP inventory update: {err}"))?;
        registry_ip_admin_call(Method::PUT, url, Some(body), "update IP inventory")
    })
    .await
    .map_err(|err| format!("IP inventory update task stopped unexpectedly: {err}"))?
}

#[tauri::command]
async fn run_ip_ping_scan() -> Result<IpPingScanResult, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let url = registry_api_url(&["ip-addresses", "ping"])?;
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(90))
            .build()
            .map_err(|err| format!("Cannot create IP ping client: {err}"))?;
        let response = client
            .post(url.clone())
            .header("X-QA-User", registry_actor())
            .send()
            .map_err(|err| format!("Cannot run IP ping at {url}: {err}"))?;
        if !response.status().is_success() {
            return Err(registry_response_error(
                "run IP ping",
                url.as_str(),
                response,
            ));
        }
        let response_body = response
            .text()
            .map_err(|err| format!("Cannot read IP ping response from {url}: {err}"))?;
        serde_json::from_str(&response_body)
            .map_err(|err| format!("The IP ping response from {url} is invalid: {err}"))
    })
    .await
    .map_err(|err| format!("IP ping task stopped unexpectedly: {err}"))?
}

#[tauri::command]
async fn run_single_ip_ping(ip_address: String) -> Result<IpPingResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        ip_address
            .parse::<Ipv4Addr>()
            .map_err(|_| format!("Invalid IP address: {ip_address}"))?;
        let url = registry_api_url(&["ip-addresses", &ip_address, "ping"])?;
        registry_ip_admin_call(Method::POST, url, None, "ping IP address")
    })
    .await
    .map_err(|err| format!("IP ping task stopped unexpectedly: {err}"))?
}

fn validate_ip_monitoring_settings(settings: &IpMonitoringSettings) -> Result<(), String> {
    if !(5..=360).contains(&settings.interval_minutes) {
        return Err("Scan interval must be between 5 and 360 minutes.".to_string());
    }
    if !(250..=10_000).contains(&settings.timeout_milliseconds) {
        return Err("Ping timeout must be between 250 and 10000 milliseconds.".to_string());
    }
    Ok(())
}

#[tauri::command]
fn get_ip_monitoring_settings() -> Result<IpMonitoringSettings, String> {
    let envelope = read_registry_setting::<IpMonitoringSettings>("ip-monitoring")?;
    validate_ip_monitoring_settings(&envelope.value)?;
    Ok(envelope.value)
}

#[tauri::command]
fn save_ip_monitoring_settings(
    settings: IpMonitoringSettings,
) -> Result<IpMonitoringSettings, String> {
    validate_ip_monitoring_settings(&settings)?;
    save_registry_setting("ip-monitoring", &settings)
}

#[tauri::command]
async fn get_ip_requests(status: Option<String>) -> Result<Vec<IpRequestRecord>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut url = registry_api_url(&["ip-requests"])?;
        if let Some(status) = status.filter(|value| !value.trim().is_empty()) {
            url.query_pairs_mut().append_pair("status", status.trim());
        }
        registry_ip_admin_call(Method::GET, url, None, "read IP requests")
    })
    .await
    .map_err(|err| format!("IP request task stopped unexpectedly: {err}"))?
}

#[tauri::command]
async fn approve_ip_request(
    request_id: String,
    request: ApproveIpRequestRequest,
) -> Result<IpRequestRecord, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let url = registry_api_url(&["ip-requests", &request_id, "approve"])?;
        let body = serde_json::to_string(&request)
            .map_err(|err| format!("Cannot prepare IP request approval: {err}"))?;
        registry_ip_admin_call(Method::POST, url, Some(body), "approve IP request")
    })
    .await
    .map_err(|err| format!("IP request approval task stopped unexpectedly: {err}"))?
}

#[tauri::command]
async fn reject_ip_request(
    request_id: String,
    request: RejectIpRequestRequest,
) -> Result<IpRequestRecord, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let url = registry_api_url(&["ip-requests", &request_id, "reject"])?;
        let body = serde_json::to_string(&request)
            .map_err(|err| format!("Cannot prepare IP request rejection: {err}"))?;
        registry_ip_admin_call(Method::POST, url, Some(body), "reject IP request")
    })
    .await
    .map_err(|err| format!("IP request rejection task stopped unexpectedly: {err}"))?
}

fn registry_actor() -> String {
    let username = std::env::var("USERNAME").unwrap_or_else(|_| "unknown".to_string());
    match std::env::var("USERDOMAIN") {
        Ok(domain) if !domain.trim().is_empty() => format!("{}\\{}", domain.trim(), username),
        _ => username,
    }
}

fn registry_http_client() -> Result<Client, String> {
    Client::builder()
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|err| format!("Cannot create receiver registry client: {err}"))
}

fn registry_file_http_client() -> Result<Client, String> {
    Client::builder()
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|err| format!("Cannot create receiver registry file client: {err}"))
}

fn registry_firmware_history_url(job_id: Option<&str>, clone_file: bool) -> Result<Url, String> {
    let mut url = Url::parse(&registry_base_url())
        .map_err(|err| format!("Receiver registry URL is invalid: {err}"))?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| "Receiver registry URL cannot contain this base path.".to_string())?;
        segments.pop_if_empty();
        segments.extend(["api", "v1", "firmware-history"]);
        if let Some(job_id) = job_id {
            segments.push(job_id);
        }
        if clone_file {
            segments.push("clone-file");
        }
    }
    Ok(url)
}

fn read_registry_firmware_history() -> Result<Vec<FirmwareSingleUpdateHistoryEntry>, String> {
    let mut url = registry_firmware_history_url(None, false)?;
    url.query_pairs_mut().append_pair("limit", "1000");
    let response = registry_http_client()?
        .get(url.clone())
        .send()
        .map_err(|err| format!("Cannot read shared firmware history from {url}: {err}"))?;
    if !response.status().is_success() {
        return Err(registry_response_error(
            "read shared firmware history",
            url.as_str(),
            response,
        ));
    }

    let body = response
        .text()
        .map_err(|err| format!("Cannot read shared firmware history response: {err}"))?;
    let entries: Vec<RegistryFirmwareHistoryEntry> = serde_json::from_str(&body)
        .map_err(|err| format!("Shared firmware history response is invalid: {err}"))?;
    Ok(entries
        .into_iter()
        .map(RegistryFirmwareHistoryEntry::into_history_entry)
        .collect())
}

fn upsert_registry_firmware_history(
    entry: &FirmwareSingleUpdateHistoryEntry,
) -> Result<RegistryFirmwareHistoryEntry, String> {
    let url = registry_firmware_history_url(Some(&entry.job_id), false)?;
    let body = serde_json::to_string(entry)
        .map_err(|err| format!("Cannot serialize firmware history entry: {err}"))?;
    let response = registry_http_client()?
        .put(url.clone())
        .header(CONTENT_TYPE, "application/json")
        .header("X-QA-User", registry_actor())
        .body(body)
        .send()
        .map_err(|err| format!("Cannot save shared firmware history to {url}: {err}"))?;
    if !response.status().is_success() {
        return Err(registry_response_error(
            "save shared firmware history",
            url.as_str(),
            response,
        ));
    }

    let body = response
        .text()
        .map_err(|err| format!("Cannot read saved firmware history response: {err}"))?;
    serde_json::from_str(&body)
        .map_err(|err| format!("Saved firmware history response is invalid: {err}"))
}

fn upload_registry_clone_file(
    job_id: &str,
    file_name: &str,
    local_path: &str,
) -> Result<(), String> {
    if local_path.starts_with("registry://") {
        return Ok(());
    }
    let source = PathBuf::from(local_path);
    if !source.is_file() {
        return Err(format!(
            "Local Clone File does not exist: {}",
            source.display()
        ));
    }
    let contents = fs::read(&source)
        .map_err(|err| format!("Cannot read local Clone File {}: {err}", source.display()))?;
    let mut url = registry_firmware_history_url(Some(job_id), true)?;
    url.query_pairs_mut().append_pair("fileName", file_name);
    let response = registry_file_http_client()?
        .put(url.clone())
        .header(CONTENT_TYPE, "application/xml")
        .header("X-QA-User", registry_actor())
        .body(contents)
        .send()
        .map_err(|err| format!("Cannot upload Clone File to {url}: {err}"))?;
    if !response.status().is_success() {
        return Err(registry_response_error(
            "upload Clone File",
            url.as_str(),
            response,
        ));
    }
    Ok(())
}

fn sync_registry_firmware_history_entry(
    entry: &FirmwareSingleUpdateHistoryEntry,
) -> Result<(), String> {
    let remote = upsert_registry_firmware_history(entry)?;
    if remote.clone_file.is_none() {
        if let (Some(file_name), Some(local_path)) = (
            entry.clone_file_name.as_deref(),
            entry.clone_file_path.as_deref(),
        ) {
            upload_registry_clone_file(&entry.job_id, file_name, local_path)?;
        }
    }
    Ok(())
}

fn delete_registry_firmware_history(job_id: &str) -> Result<(), String> {
    let url = registry_firmware_history_url(Some(job_id), false)?;
    let response = registry_http_client()?
        .delete(url.clone())
        .header("X-QA-User", registry_actor())
        .send()
        .map_err(|err| format!("Cannot delete shared firmware history at {url}: {err}"))?;
    if !response.status().is_success() {
        return Err(registry_response_error(
            "delete shared firmware history",
            url.as_str(),
            response,
        ));
    }
    Ok(())
}

fn download_registry_clone_file(job_id: &str) -> Result<CloneFileDownloadResult, String> {
    let url = registry_firmware_history_url(Some(job_id), true)?;
    let response = registry_file_http_client()?
        .get(url.clone())
        .send()
        .map_err(|err| format!("Cannot download shared Clone File from {url}: {err}"))?;
    if !response.status().is_success() {
        return Err(registry_response_error(
            "download shared Clone File",
            url.as_str(),
            response,
        ));
    }

    let file_name = response
        .headers()
        .get("X-QA-File-Name")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
        .unwrap_or_else(|| format!("CloneFile-{job_id}.xml"));
    if Path::new(&file_name)
        .file_name()
        .and_then(|value| value.to_str())
        != Some(&file_name)
    {
        return Err("Shared Clone File name is invalid.".to_string());
    }
    let expected_sha256 = response
        .headers()
        .get("X-QA-SHA256")
        .and_then(|value| value.to_str().ok())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| "Shared Clone File response does not contain a checksum.".to_string())?;
    let contents = response
        .bytes()
        .map_err(|err| format!("Cannot read shared Clone File response: {err}"))?;
    let actual_sha256 = format!("{:x}", Sha256::digest(&contents));
    if actual_sha256 != expected_sha256 {
        return Err("Shared Clone File checksum verification failed.".to_string());
    }
    let clone_text = String::from_utf8_lossy(&contents);
    if !is_valid_clone_file_response(&clone_text) {
        return Err("Shared Clone File is incomplete or invalid.".to_string());
    }

    let destination = next_available_download_path(&file_name)?;
    fs::write(&destination, &contents).map_err(|err| {
        format!(
            "Cannot save Clone File to Downloads {}: {err}",
            destination.display()
        )
    })?;
    Ok(CloneFileDownloadResult {
        file_name,
        destination_path: destination.display().to_string(),
    })
}

fn registry_response_error(
    action: &str,
    url: &str,
    response: reqwest::blocking::Response,
) -> String {
    let status = response.status();
    let body = response.text().unwrap_or_default();
    let detail = serde_json::from_str::<Value>(&body).ok().and_then(|value| {
        value
            .get("detail")
            .and_then(Value::as_str)
            .map(str::to_string)
    });
    match detail {
        Some(detail) => format!("Cannot {action} at {url}: HTTP {status}: {detail}"),
        None => format!("Cannot {action} at {url}: HTTP {status}."),
    }
}

fn read_registry_setting<T: DeserializeOwned>(
    key: &str,
) -> Result<RegistrySettingEnvelope<T>, String> {
    let url = registry_setting_url(key);
    let response = registry_http_client()?
        .get(&url)
        .send()
        .map_err(|err| format!("Cannot read shared setting from {url}: {err}"))?;
    if !response.status().is_success() {
        return Err(registry_response_error(
            "read shared setting",
            &url,
            response,
        ));
    }

    let body = response
        .text()
        .map_err(|err| format!("Cannot read shared setting response from {url}: {err}"))?;
    serde_json::from_str(&body)
        .map_err(|err| format!("Shared setting response from {url} is invalid: {err}"))
}

fn save_registry_setting<T: Serialize + DeserializeOwned>(
    key: &str,
    value: &T,
) -> Result<T, String> {
    let current: RegistrySettingEnvelope<T> = read_registry_setting(key)?;
    let request = RegistrySettingUpdate {
        value,
        row_version: current.row_version,
    };
    let body = serde_json::to_string(&request)
        .map_err(|err| format!("Cannot serialize shared setting {key}: {err}"))?;
    let url = registry_setting_url(key);
    let response = registry_http_client()?
        .put(&url)
        .header(CONTENT_TYPE, "application/json")
        .header("X-QA-User", registry_actor())
        .body(body)
        .send()
        .map_err(|err| format!("Cannot save shared setting to {url}: {err}"))?;
    if !response.status().is_success() {
        return Err(registry_response_error(
            "save shared setting",
            &url,
            response,
        ));
    }

    let body = response
        .text()
        .map_err(|err| format!("Cannot read saved setting response from {url}: {err}"))?;
    let saved: RegistrySettingEnvelope<T> = serde_json::from_str(&body)
        .map_err(|err| format!("Saved setting response from {url} is invalid: {err}"))?;
    Ok(saved.value)
}

fn read_update_safety_settings() -> Result<UpdateSafetySettings, String> {
    match read_registry_setting::<UpdateSafetySettings>("update-safety") {
        Ok(envelope) => {
            let mut settings = envelope.value;
            settings.restart_cycle_minutes = CONVERGENCE_RESTART_CYCLE_MINUTES;
            validate_update_safety_settings(&settings)?;
            if let Err(error) = write_local_update_safety_settings(&settings) {
                log::warn!("Cannot refresh local Update Safety cache: {error}");
            }
            Ok(settings)
        }
        Err(error) => {
            log::warn!("Shared Update Safety setting is unavailable; using local cache: {error}");
            read_local_update_safety_settings()
        }
    }
}

fn read_local_update_safety_settings() -> Result<UpdateSafetySettings, String> {
    let path = update_safety_settings_path()?;
    if !path.is_file() {
        return Ok(UpdateSafetySettings::default());
    }

    let text = fs::read_to_string(&path).map_err(|err| {
        format!(
            "Cannot read update safety settings {}: {err}",
            path.display()
        )
    })?;
    let mut settings: UpdateSafetySettings = serde_json::from_str(&text)
        .map_err(|err| format!("Invalid update safety settings {}: {err}", path.display()))?;
    settings.restart_cycle_minutes = CONVERGENCE_RESTART_CYCLE_MINUTES;
    validate_update_safety_settings(&settings)?;
    Ok(settings)
}

fn write_update_safety_settings(settings: &UpdateSafetySettings) -> Result<(), String> {
    let saved = save_registry_setting("update-safety", settings)?;
    write_local_update_safety_settings(&saved)
}

fn write_local_update_safety_settings(settings: &UpdateSafetySettings) -> Result<(), String> {
    let path = update_safety_settings_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "Cannot create update safety settings directory {}: {err}",
                parent.display()
            )
        })?;
    }
    let text = serde_json::to_string_pretty(settings)
        .map_err(|err| format!("Cannot serialize update safety settings: {err}"))?;
    fs::write(&path, text).map_err(|err| {
        format!(
            "Cannot write update safety settings {}: {err}",
            path.display()
        )
    })
}

fn format_safety_duration(total_seconds: i64) -> String {
    let seconds = total_seconds.max(0);
    let minutes = seconds / 60;
    let remainder = seconds % 60;
    if minutes > 0 {
        format!("{minutes}m {remainder:02}s")
    } else {
        format!("{remainder}s")
    }
}

fn evaluate_update_safety_window(
    ip: &str,
    uptime_seconds: i64,
    settings: &UpdateSafetySettings,
) -> UpdateSafetyEvaluation {
    let uptime = uptime_seconds.max(0);
    let cycle_seconds = i64::from(CONVERGENCE_RESTART_CYCLE_MINUTES) * 60;
    let post_restart_seconds = i64::from(settings.post_restart_delay_minutes) * 60;
    let pre_restart_seconds = i64::from(settings.pre_restart_safety_margin_minutes) * 60;
    let safe_window_end = cycle_seconds - pre_restart_seconds;

    if uptime < post_restart_seconds {
        let wait = post_restart_seconds - uptime;
        return UpdateSafetyEvaluation {
            ip: ip.to_string(),
            allowed: false,
            state: "PostRestartDelay".to_string(),
            message: format!(
                "Receiver restarted recently. Update will be available in {}.",
                format_safety_duration(wait)
            ),
            uptime_seconds: uptime,
            available_in_seconds: Some(wait),
            window_closes_in_seconds: None,
        };
    }

    if uptime < safe_window_end {
        let remaining = safe_window_end - uptime;
        return UpdateSafetyEvaluation {
            ip: ip.to_string(),
            allowed: true,
            state: "SafeWindow".to_string(),
            message: format!(
                "Receiver is inside the safe update window for another {}.",
                format_safety_duration(remaining)
            ),
            uptime_seconds: uptime,
            available_in_seconds: None,
            window_closes_in_seconds: Some(remaining),
        };
    }

    if uptime < cycle_seconds {
        let wait = cycle_seconds - uptime;
        return UpdateSafetyEvaluation {
            ip: ip.to_string(),
            allowed: false,
            state: "ScheduledRestartMargin".to_string(),
            message: format!(
                "Receiver is approaching its scheduled restart. Update will be available in {} if the receiver remains online.",
                format_safety_duration(wait)
            ),
            uptime_seconds: uptime,
            available_in_seconds: Some(wait),
            window_closes_in_seconds: None,
        };
    }

    UpdateSafetyEvaluation {
        ip: ip.to_string(),
        allowed: true,
        state: "CycleExceeded".to_string(),
        message: format!(
            "Receiver uptime is at least {} minutes. The convergence restart restriction no longer applies.",
            CONVERGENCE_RESTART_CYCLE_MINUTES
        ),
        uptime_seconds: uptime,
        available_in_seconds: None,
        window_closes_in_seconds: None,
    }
}

fn read_update_safety_uptime(
    client: &Client,
    receiver: &FirmwarePlanReceiver,
) -> Result<i64, String> {
    let ip = &receiver.credential.addr;
    let text = get_xml_text(
        client,
        &receiver.credential,
        &format!("http://{ip}/xml/dynamic/merge.xml?power="),
    )?;
    let doc = roxmltree::Document::parse(&text)
        .map_err(|err| format!("Invalid receiver power XML from {ip}: {err}"))?;
    get_uptime_seconds(&doc)
        .ok_or_else(|| format!("Receiver {ip} did not provide a readable uptime value."))
}

fn ensure_update_safety_for_receivers(receivers: &[FirmwarePlanReceiver]) -> Result<(), String> {
    for receiver in receivers {
        password_recovery_notices::ensure_ready(&receiver.credential.addr)?;
    }
    let settings = read_update_safety_settings()?;
    let client = Client::builder()
        .timeout(Duration::from_secs(8))
        .connect_timeout(Duration::from_secs(4))
        .build()
        .map_err(|err| format!("HTTP client error: {err}"))?;
    let mut blocked = Vec::new();

    for batch in receivers.chunks(DEFAULT_WORKERS.max(1)) {
        let evaluations = thread::scope(|scope| {
            let handles: Vec<_> = batch
                .iter()
                .map(|receiver| {
                    let client = client.clone();
                    let settings = settings.clone();
                    scope.spawn(move || {
                        let access = receiver_password::operation_lock(&receiver.credential.addr)?;
                        let _access = access
                            .try_lock()
                            .map_err(|_| "Receiver is busy with another operation.".to_string())?;
                        password_recovery_notices::ensure_ready(&receiver.credential.addr)?;
                        read_update_safety_uptime(&client, receiver).map(|uptime| {
                            evaluate_update_safety_window(
                                &receiver.credential.addr,
                                uptime,
                                &settings,
                            )
                        })
                    })
                })
                .collect();

            handles
                .into_iter()
                .map(|handle| {
                    handle.join().unwrap_or_else(|_| {
                        Err("Update Safety check stopped unexpectedly.".to_string())
                    })
                })
                .collect::<Vec<_>>()
        });

        for (receiver, evaluation) in batch.iter().zip(evaluations) {
            match evaluation {
                Ok(result) if result.allowed => {}
                Ok(result) => {
                    blocked.push(format!("{}: {}", receiver.credential.addr, result.message))
                }
                Err(error) => blocked.push(format!(
                    "{}: Update Safety could not verify uptime. {}",
                    receiver.credential.addr, error
                )),
            }
        }
    }

    if blocked.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Update Safety blocked this operation. {}",
            blocked.join(" ")
        ))
    }
}

fn ensure_update_safety_with_known_uptime(
    ip: &str,
    uptime_seconds: Option<i64>,
) -> Result<UpdateSafetyEvaluation, String> {
    let uptime = uptime_seconds
        .ok_or_else(|| format!("Update Safety could not verify uptime for receiver {ip}."))?;
    let settings = read_update_safety_settings()?;
    let evaluation = evaluate_update_safety_window(ip, uptime, &settings);
    if evaluation.allowed {
        Ok(evaluation)
    } else {
        Err(format!(
            "Update Safety blocked {ip}. {}",
            evaluation.message
        ))
    }
}

fn read_single_update_history() -> Result<Vec<FirmwareSingleUpdateHistoryEntry>, String> {
    let path = single_update_history_path()?;
    read_firmware_history_file(&path)
}

fn read_registry_history_cache() -> Result<Vec<FirmwareSingleUpdateHistoryEntry>, String> {
    let path = registry_history_cache_path()?;
    read_firmware_history_file(&path)
}

fn read_firmware_history_file(
    path: &Path,
) -> Result<Vec<FirmwareSingleUpdateHistoryEntry>, String> {
    if !path.is_file() {
        return Ok(Vec::new());
    }

    let text = fs::read_to_string(&path).map_err(|err| {
        format!(
            "Cannot read single update history {}: {err}",
            path.display()
        )
    })?;
    let mut history: Vec<FirmwareSingleUpdateHistoryEntry> = serde_json::from_str(&text)
        .map_err(|err| format!("Invalid single update history {}: {err}", path.display()))?;
    history.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(history)
}

fn append_single_update_history(entry: FirmwareSingleUpdateHistoryEntry) -> Result<(), String> {
    let mut history = read_single_update_history().unwrap_or_default();
    history.retain(|existing| existing.job_id != entry.job_id);
    history.insert(0, entry.clone());
    history.truncate(200);
    write_single_update_history(&history)?;
    if let Err(error) = sync_registry_firmware_history_entry(&entry) {
        log::warn!("Cannot synchronize completed firmware update: {error}");
    }
    Ok(())
}

fn write_single_update_history(history: &[FirmwareSingleUpdateHistoryEntry]) -> Result<(), String> {
    let path = single_update_history_path()?;
    write_firmware_history_file(&path, history)
}

fn write_registry_history_cache(
    history: &[FirmwareSingleUpdateHistoryEntry],
) -> Result<(), String> {
    let path = registry_history_cache_path()?;
    write_firmware_history_file(&path, history)
}

fn write_firmware_history_file(
    path: &Path,
    history: &[FirmwareSingleUpdateHistoryEntry],
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "Cannot create history directory {}: {err}",
                parent.display()
            )
        })?;
    }
    let text = serde_json::to_string_pretty(&history)
        .map_err(|err| format!("Cannot serialize single update history: {err}"))?;
    fs::write(&path, text).map_err(|err| {
        format!(
            "Cannot write single update history {}: {err}",
            path.display()
        )
    })
}

fn merge_firmware_history(
    primary: Vec<FirmwareSingleUpdateHistoryEntry>,
    fallback: Vec<FirmwareSingleUpdateHistoryEntry>,
) -> Vec<FirmwareSingleUpdateHistoryEntry> {
    let mut seen = BTreeSet::new();
    let mut merged = Vec::with_capacity(primary.len() + fallback.len());
    for entry in primary.into_iter().chain(fallback) {
        if seen.insert(entry.job_id.clone()) {
            merged.push(entry);
        }
    }
    merged.sort_by(|left, right| right.started_at.cmp(&left.started_at));
    merged.truncate(1000);
    merged
}

fn delete_clone_file_backup(path: &str) -> Result<(), String> {
    let source = PathBuf::from(path.trim());
    if !source.exists() {
        return Ok(());
    }

    let clone_root = qa_receiver_center_data_dir()?.join("clone-files");
    let source_canonical = source
        .canonicalize()
        .map_err(|err| format!("Cannot resolve clone file path: {err}"))?;
    let clone_root_canonical = clone_root
        .canonicalize()
        .map_err(|err| format!("Cannot resolve clone file backup directory: {err}"))?;
    if !source_canonical.starts_with(&clone_root_canonical) {
        return Err("Clone file path is outside QA Receiver Center backups.".to_string());
    }

    fs::remove_file(&source_canonical)
        .map_err(|err| format!("Cannot delete clone file backup: {err}"))?;

    if let Some(job_dir) = source_canonical.parent() {
        let _ = fs::remove_dir(job_dir);
        if let Some(receiver_dir) = job_dir.parent() {
            let _ = fs::remove_dir(receiver_dir);
        }
    }

    Ok(())
}

fn verify_receiver_password(rx: &ReceiverCredential, password: &str) -> Result<(), String> {
    receiver_password::verify(rx, password)
}

fn format_workflow_state(value: &str) -> String {
    match value {
        "CheckPassword" => "Check Password".to_string(),
        "ChangePassword" => "Change Password".to_string(),
        "UploadFirmware" => "Upload Firmware".to_string(),
        "MonitorInstall" => "Monitor Install".to_string(),
        "RestorePassword" => "Restore Password".to_string(),
        "VerifyResult" => "Verify Result".to_string(),
        other => other.to_string(),
    }
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn now_seconds() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

fn today_clone_date() -> String {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 86_400;
    let (year, month, day) = civil_from_days(days as i64);
    format!("{day:02}_{month:02}_{year:04}")
}

fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32, day as u32)
}

fn read_receivers(path: &str) -> Result<Vec<ReceiverCredential>, String> {
    let raw = if path == GROUPS_JSON_PATH {
        match read_receivers_json_from_ado() {
            Ok(raw) => raw,
            Err(ado_error) => fs::read_to_string(path).map_err(|file_error| {
                format!(
                    "Cannot load qa-receivers.json from ADO ({ado_error}) or the network fallback ({file_error})."
                )
            })?,
        }
    } else {
        fs::read_to_string(path).map_err(|err| format!("Cannot read receiver JSON: {err}"))?
    };
    parse_receivers_json(&raw)
}

#[cfg(target_os = "windows")]
fn read_receivers_json_from_ado() -> Result<String, String> {
    let ado_url =
        std::env::var("QA_RECEIVERS_ADO_URL").unwrap_or_else(|_| GROUPS_ADO_API_URL.to_string());
    let script = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$response = Invoke-WebRequest -UseBasicParsing -UseDefaultCredentials -TimeoutSec 20 -Headers @{ Accept = 'application/json' } -Uri $env:QA_RECEIVERS_ADO_REQUEST_URL
$payload = [string]$response.Content
try {
  $item = $payload | ConvertFrom-Json
  if ($null -ne $item.content) {
    $payload = [string]$item.content
  }
} catch {
  # ADO can also return the file body directly.
}
[Console]::Out.Write($payload)
"#;
    let output = Command::new("powershell.exe")
        .env("QA_RECEIVERS_ADO_REQUEST_URL", ado_url)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .output()
        .map_err(|err| format!("Cannot start the ADO receiver configuration request: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "ADO receiver configuration request failed.".to_string()
        } else {
            format!("ADO receiver configuration request failed: {stderr}")
        });
    }

    let raw = String::from_utf8(output.stdout)
        .map_err(|err| format!("ADO receiver configuration is not valid UTF-8: {err}"))?;
    if raw.trim().is_empty() {
        return Err("ADO returned an empty receiver configuration.".to_string());
    }
    Ok(raw)
}

#[cfg(not(target_os = "windows"))]
fn read_receivers_json_from_ado() -> Result<String, String> {
    Err("ADO integrated authentication is currently available on Windows only.".to_string())
}

fn parse_receivers_json(raw: &str) -> Result<Vec<ReceiverCredential>, String> {
    let root: Value =
        serde_json::from_str(raw).map_err(|err| format!("Invalid receiver JSON: {err}"))?;

    let mut found = Vec::new();
    walk_receiver_nodes(&root, &mut found);

    let mut unique: HashMap<String, ReceiverCredential> = HashMap::new();
    for rx in found {
        unique.entry(rx.addr.clone()).or_insert(rx);
    }

    let mut receivers: Vec<_> = unique.into_values().collect();
    receivers.sort_by_key(|receiver| ip_sort_key(&receiver.addr));
    Ok(receivers)
}

fn walk_receiver_nodes(value: &Value, found: &mut Vec<ReceiverCredential>) {
    match value {
        Value::Array(items) => {
            for item in items {
                walk_receiver_nodes(item, found);
            }
        }
        Value::Object(map) => {
            let addr = map.get("addr").and_then(Value::as_str).map(str::trim);
            let user = map.get("user").and_then(Value::as_str);
            let pw = map.get("pw").and_then(Value::as_str);
            let group = map.get("group").and_then(|v| {
                v.as_u64()
                    .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
            });

            if let (Some(addr), Some(user), Some(pw), Some(group)) = (addr, user, pw, group) {
                if !addr.is_empty() && !user.trim().is_empty() && (1..=4).contains(&group) {
                    found.push(ReceiverCredential {
                        addr: addr.to_string(),
                        user: user.to_string(),
                        pw: pw.to_string(),
                        group: group as u8,
                    });
                }
            }

            for child in map.values() {
                walk_receiver_nodes(child, found);
            }
        }
        _ => {}
    }
}

fn ip_sort_key(ip: &str) -> (u8, [u8; 4], String) {
    match ip.parse::<Ipv4Addr>() {
        Ok(addr) => (0, addr.octets(), String::new()),
        Err(_) => (1, [255; 4], ip.to_string()),
    }
}

fn scan_all(
    receivers: Vec<ReceiverCredential>,
    single_jobs: &Arc<Mutex<HashMap<String, FirmwareSingleUpdateJobStatus>>>,
) -> Vec<ReceiverSummary> {
    let queue = Arc::new(Mutex::new(VecDeque::from(receivers.clone())));
    let results = Arc::new(Mutex::new(Vec::with_capacity(receivers.len())));
    let workers = DEFAULT_WORKERS.min(receivers.len().max(1));

    thread::scope(|scope| {
        for _ in 0..workers {
            let queue = Arc::clone(&queue);
            let results = Arc::clone(&results);

            scope.spawn(move || loop {
                let next = {
                    let mut guard = queue.lock().expect("receiver queue mutex poisoned");
                    guard.pop_front()
                };

                let Some(rx) = next else {
                    break;
                };

                let access = receiver_password::operation_lock(&rx.addr).ok();
                let access_guard = access.as_ref().and_then(|access| access.try_lock().ok());
                let paused = access_guard.is_none()
                    || password_recovery_notices::read()
                        .map(|notices| notices.iter().any(|notice| notice.ip == rx.addr))
                        .unwrap_or(true)
                    || single_jobs
                        .lock()
                        .map(|jobs| {
                            jobs.values()
                                .any(|job| job.ip == rx.addr && job.completed_at.is_none())
                        })
                        .unwrap_or(true);
                let summary = if paused {
                    let mut summary = ReceiverSummary::from_config(&rx);
                    summary.needs_review = true;
                    summary.last_error = Some(
                        "Status scan paused during firmware update or pending password recovery."
                            .into(),
                    );
                    summary
                } else {
                    scan_receiver(&rx)
                };
                let mut guard = results.lock().expect("receiver results mutex poisoned");
                guard.push(summary);
            });
        }
    });

    let mut summaries = Arc::try_unwrap(results)
        .expect("receiver results still referenced")
        .into_inner()
        .expect("receiver results mutex poisoned");
    summaries.sort_by_key(|summary| ip_sort_key(&summary.ip));
    summaries
}

fn scan_receiver(rx: &ReceiverCredential) -> ReceiverSummary {
    let client = match Client::builder()
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(4))
        .build()
    {
        Ok(client) => client,
        Err(err) => return ReceiverSummary::offline(rx, format!("HTTP client error: {err}")),
    };

    let sys_text = match get_xml_text(
        &client,
        rx,
        &format!("http://{}/xml/dynamic/merge.xml?sysData=", rx.addr),
    ) {
        Ok(text) => text,
        Err(err) => return ReceiverSummary::offline(rx, err),
    };
    let sys_doc = match roxmltree::Document::parse(&sys_text) {
        Ok(doc) => doc,
        Err(err) => {
            return ReceiverSummary::offline(rx, format!("Invalid XML from {}: {err}", rx.addr))
        }
    };

    let rx_name = select_text(&sys_doc, "RXName").map(normalize_receiver_name);
    let fw_version = select_text(&sys_doc, "FWVersion");
    let fw_date = select_text(&sys_doc, "FWDate");
    let rtk_version = select_text(&sys_doc, "RTKversion").map(clean_trailing_non_alnum);
    let test_mode = select_text(&sys_doc, "testMode").and_then(|value| parse_bool(&value));
    let power_text = get_xml_text(
        &client,
        rx,
        &format!("http://{}/xml/dynamic/merge.xml?power=", rx.addr),
    )
    .ok();
    let pos_text = get_xml_text(
        &client,
        rx,
        &format!("http://{}/xml/dynamic/merge.xml?pos=&", rx.addr),
    )
    .ok();
    let err_text = get_xml_text(
        &client,
        rx,
        &format!(
            "http://{}/xml/dynamic/merge.xml?errLog=&procNames=",
            rx.addr
        ),
    )
    .or_else(|_| {
        get_xml_text(
            &client,
            rx,
            &format!(
                "http://{}/xml/dynamic/sec_merge.xml?errLog=&procNames=",
                rx.addr
            ),
        )
    })
    .ok();

    let (uptime_seconds, celsius) = power_text
        .as_deref()
        .and_then(|text| roxmltree::Document::parse(text).ok())
        .map(|doc| {
            (
                get_uptime_seconds(&doc),
                select_text(&doc, "celsius").and_then(|value| parse_float(&value)),
            )
        })
        .unwrap_or((None, None));

    let (sv_solved, fix_type) = pos_text
        .as_deref()
        .and_then(|text| roxmltree::Document::parse(text).ok())
        .map(|doc| {
            (
                select_text(&doc, "numFixSvs").and_then(|value| parse_float(&value)),
                select_text(&doc, "fixType").map(clean_fix_type),
            )
        })
        .unwrap_or((None, None));

    let error_count = err_text
        .as_deref()
        .and_then(|text| roxmltree::Document::parse(text).ok())
        .map(|doc| count_receiver_error_entries(&doc));
    let error_warning = error_count.map(|count| count as f64);
    let needs_review = error_count.unwrap_or(0) > 0;

    ReceiverSummary {
        ip: rx.addr.clone(),
        group: rx.group,
        status: "online".to_string(),
        receiver_name: rx_name,
        firmware_version: fw_version,
        firmware_date: fw_date,
        rtk_version,
        test_mode,
        uptime_seconds,
        celsius,
        sv_solved,
        fix_type,
        error_warning,
        needs_review,
        last_error: None,
    }
}

fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map_err(|err| format!("Cannot open receiver page: {err}"))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|err| format!("Cannot open receiver page: {err}"))?;
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|err| format!("Cannot open receiver page: {err}"))?;
    }

    Ok(())
}

fn is_receiver_authentication_error(error: &str) -> bool {
    [401, 403, 423, 429]
        .iter()
        .any(|status| error.starts_with(&format!("HTTP {status} for ")))
}

fn get_xml_text(client: &Client, rx: &ReceiverCredential, url: &str) -> Result<String, String> {
    get_xml_text_with_password(client, rx, &rx.pw, url)
}

fn get_xml_text_with_password(
    client: &Client,
    rx: &ReceiverCredential,
    password: &str,
    url: &str,
) -> Result<String, String> {
    let response = client
        .get(url)
        .basic_auth(&rx.user, Some(password))
        .send()
        .map_err(|err| format!("Request failed for {}: {}", rx.addr, clean_error(err)))?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP {} for {}", status.as_u16(), rx.addr));
    }

    let text = response.text().map_err(|err| {
        format!(
            "Cannot read response from {}: {}",
            rx.addr,
            clean_error(err)
        )
    })?;
    Ok(text)
}

fn clean_error(err: reqwest::Error) -> String {
    if err.is_timeout() {
        "timeout".to_string()
    } else if err.is_connect() {
        "connection failed".to_string()
    } else {
        err.without_url().to_string()
    }
}

fn count_receiver_error_entries(doc: &roxmltree::Document) -> usize {
    doc.descendants()
        .filter(|node| node.is_element() && node.tag_name().name().eq_ignore_ascii_case("entry"))
        .filter(|entry| {
            select_text_from_node(*entry, "flags")
                .and_then(|value| value.parse::<u32>().ok())
                .is_some_and(|flags| flags & 0x7 == 0)
        })
        .count()
}

fn select_text(doc: &roxmltree::Document, tag_name: &str) -> Option<String> {
    doc.descendants()
        .find(|node| node.is_element() && node.tag_name().name().eq_ignore_ascii_case(tag_name))
        .and_then(|node| node.text())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn find_node<'a, 'input>(
    doc: &'a roxmltree::Document<'input>,
    tag_name: &str,
) -> Option<roxmltree::Node<'a, 'input>> {
    doc.descendants()
        .find(|node| node.is_element() && node.tag_name().name().eq_ignore_ascii_case(tag_name))
}

fn select_text_from_node(node: roxmltree::Node<'_, '_>, tag_name: &str) -> Option<String> {
    node.descendants()
        .skip(1)
        .find(|child| child.is_element() && child.tag_name().name().eq_ignore_ascii_case(tag_name))
        .and_then(|child| child.text())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn select_nested_text_from_node(node: roxmltree::Node<'_, '_>, path: &[&str]) -> Option<String> {
    let mut current = node;
    for tag_name in path {
        current = current.descendants().skip(1).find(|child| {
            child.is_element() && child.tag_name().name().eq_ignore_ascii_case(tag_name)
        })?;
    }
    current
        .text()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn render_node_text(node: roxmltree::Node<'_, '_>) -> Option<String> {
    let text = node.text().map(str::trim).filter(|value| !value.is_empty());
    if text.is_some() {
        return text.map(ToOwned::to_owned);
    }

    let values = node
        .descendants()
        .skip(1)
        .filter_map(|child| child.text())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some(values.join(" / "))
    }
}

fn render_receiver_fw_status(
    mode: Option<&str>,
    percent: Option<f64>,
    in_progress: bool,
) -> Option<String> {
    let mode = mode?.trim();
    if mode.is_empty() {
        return None;
    }

    if in_progress {
        if let Some(percent) = percent {
            return Some(format!("{mode} ({})", format_percent(percent)));
        }
    }

    Some(mode.to_string())
}

fn get_uptime_seconds(doc: &roxmltree::Document) -> Option<i64> {
    let days = select_text(doc, "day")?.parse::<i64>().ok()?;
    let hours = select_text(doc, "hour")?.parse::<i64>().ok()?;
    let minutes = select_text(doc, "min")?.parse::<i64>().ok()?;
    let seconds = select_text(doc, "sec")?.parse::<i64>().ok()?;
    Some(days * 86_400 + hours * 3_600 + minutes * 60 + seconds)
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_uppercase().as_str() {
        "TRUE" => Some(true),
        "FALSE" => Some(false),
        _ => None,
    }
}

fn parse_float(value: &str) -> Option<f64> {
    value.trim().parse::<f64>().ok()
}

fn parse_percent(value: &str) -> Option<f64> {
    value
        .trim()
        .trim_end_matches('%')
        .trim()
        .parse::<f64>()
        .ok()
}

fn format_percent(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{}%", value as i64)
    } else {
        format!("{value:.1}%")
    }
}

fn clean_trailing_non_alnum(value: String) -> String {
    value
        .trim()
        .trim_end_matches(|ch: char| !ch.is_ascii_alphanumeric())
        .trim()
        .to_string()
}

fn clean_fix_type(value: String) -> String {
    value
        .replace('\\', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_receiver_name(value: String) -> String {
    value
        .trim()
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '\\' && *ch != '/')
        .collect()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .on_page_load(|webview, payload| {
            if payload.event() == PageLoadEvent::Finished {
                let _ = webview.set_zoom(DEFAULT_INTERFACE_ZOOM);
            }
        })
        .manage(FirmwareJobs::default())
        .invoke_handler(tauri::generate_handler![
            load_receivers_config,
            scan_receivers,
            get_ip_inventory,
            update_ip_inventory,
            run_ip_ping_scan,
            run_single_ip_ping,
            get_ip_requests,
            approve_ip_request,
            reject_ip_request,
            get_update_safety_settings,
            save_update_safety_settings,
            get_ip_monitoring_settings,
            save_ip_monitoring_settings,
            authenticate_operator,
            open_receiver,
            get_receiver_login_details,
            get_firmware_update_config,
            list_firmware_releases,
            list_firmware_builds,
            inspect_firmware_release,
            add_firmware_config_receiver,
            remove_firmware_config_receiver,
            validate_firmware_plan,
            dry_run_single_firmware_update,
            pick_local_firmware_package,
            pick_local_firmware_packages,
            inspect_local_firmware_package,
            start_single_local_firmware_update,
            start_single_release_firmware_update_job,
            start_single_local_firmware_update_job,
            get_single_firmware_update_job_status,
            cancel_single_firmware_update_job,
            get_single_firmware_update_history,
            delete_single_firmware_update_history,
            download_clone_file_backup,
            restore_receiver_password,
            get_password_recovery_notices,
            play_password_recovery_sound,
            force_restore_firmware_job_passwords,
            start_firmware_update_job,
            get_firmware_job_status,
            stop_firmware_update_job,
            resolve_firmware_downgrade
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn password_test_job(batch: Option<&str>) -> FirmwareSingleUpdateJobStatus {
        create_single_update_job_status(
            "test-job",
            &FirmwarePlanReceiver {
                credential: ReceiverCredential {
                    addr: "127.0.0.1".into(),
                    user: "test-user".into(),
                    pw: "test-only".into(),
                    group: 1,
                },
                model: "alloy".into(),
                target_file: "test.timg".into(),
                target_url: "local://test.timg".into(),
                local_package_path: None,
            },
            batch.map(str::to_string),
            "Test queued",
        )
    }

    #[test]
    fn recovery_alert_waits_for_completion_and_includes_receiver_and_failure_details() {
        let mut job = password_test_job(Some("group-fw-test"));
        job.receiver_name = Some("Lab receiver".into());
        let notice = password_recovery_notices::PasswordRecoveryNotice {
            ip: job.ip.clone(),
            job_id: job.job_id.clone(),
            message: "Recovery is unconfirmed".into(),
        };
        let mut jobs = HashMap::from([(job.job_id.clone(), job)]);
        assert!(visible_password_recovery_notices(vec![notice.clone()], &jobs).is_empty());
        let job = jobs.get_mut("test-job").unwrap();
        push_workflow_step(
            &mut job.steps,
            "RestorePassword",
            "Failed",
            "PASSWORD RECOVERY REQUIRED: single login was rejected",
        );
        finish_single_job(job, "Failed", "Update failed");
        let alerts = visible_password_recovery_notices(vec![notice.clone()], &jobs);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].receiver_name.as_deref(), Some("Lab receiver"));
        assert_eq!(alerts[0].model.as_deref(), Some("alloy"));
        assert!(alerts[0].message.contains("single login was rejected"));
        assert!(visible_password_recovery_notices(vec![], &jobs).is_empty());
        let after_restart = visible_password_recovery_notices(vec![notice], &HashMap::new());
        assert_eq!(after_restart.len(), 1);
        assert_eq!(after_restart[0].message, "Recovery is unconfirmed");
    }

    #[test]
    fn single_group_and_multiple_cannot_complete_with_unverified_password() {
        for batch in [None, Some("group-fw-test"), Some("multiple-fw-test")] {
            for requested_status in ["Completed", "Cancelled", "Failed"] {
                let mut job = password_test_job(batch);
                push_workflow_step(
                    &mut job.steps,
                    "ChangePassword",
                    "Completed",
                    "Temporary password active",
                );
                push_workflow_step(
                    &mut job.steps,
                    "RestorePassword",
                    "Failed",
                    "PASSWORD RECOVERY REQUIRED for 127.0.0.1. Password rejected.",
                );
                finish_single_job(&mut job, requested_status, "Generic completion message");
                assert_eq!(job.status, "Failed");
                assert_eq!(job.phase, "RestorePassword");
                assert!(job.message.contains("PASSWORD RECOVERY REQUIRED"));
                let history = single_update_history_entry_from_job(&job);
                assert_eq!(history.result, "Failed");
                assert!(history
                    .errors
                    .iter()
                    .any(|error| error.contains("PASSWORD RECOVERY REQUIRED")));
            }
        }
    }

    #[test]
    fn missing_recovery_step_is_never_success() {
        let mut job = password_test_job(None);
        push_workflow_step(
            &mut job.steps,
            "ChangePassword",
            "Running",
            "Write may have started",
        );
        finish_single_job(&mut job, "Completed", "Done");
        assert_eq!(job.status, "Failed");
        assert!(job.message.starts_with("PASSWORD RECOVERY REQUIRED"));
    }

    #[test]
    fn unprotected_receiver_can_complete_without_password_recovery() {
        for batch in [None, Some("group-fw-test"), Some("multiple-fw-test")] {
            let mut job = password_test_job(batch);
            push_workflow_step(
                &mut job.steps,
                "ChangePassword",
                "Skipped",
                "Receiver security is disabled. No temporary password was set.",
            );
            push_workflow_step(
                &mut job.steps,
                "RestorePassword",
                "Skipped",
                "No password recovery was required.",
            );

            finish_single_job(&mut job, "Completed", "Update completed");

            assert_eq!(job.status, "Completed");
            assert!(password_recovery_error(&job).is_none());
        }
    }

    #[test]
    fn verified_recovery_allows_completion_and_cancellation() {
        for status in ["Completed", "Cancelled"] {
            let mut job = password_test_job(None);
            push_workflow_step(&mut job.steps, "ChangePassword", "Completed", "Active");
            push_workflow_step(&mut job.steps, "RestorePassword", "Completed", "Verified");
            finish_single_job(&mut job, status, "Verified");
            assert_eq!(job.status, status);
        }
    }

    #[test]
    fn failed_steps_keep_polling_until_recovery_finishes() {
        let jobs = Arc::new(Mutex::new(HashMap::from([(
            "test-job".into(),
            password_test_job(None),
        )])));
        update_single_job_step(
            &jobs,
            "test-job",
            "UploadFirmware",
            "Failed",
            "Failed",
            "Upload failed",
        );
        assert!(!single_job_is_terminal(&jobs, "test-job"));
        assert_eq!(
            jobs.lock().unwrap()["test-job"]
                .steps
                .last()
                .unwrap()
                .status,
            "Failed"
        );
        update_single_job_step(
            &jobs,
            "test-job",
            "RestorePassword",
            "Completed",
            "Completed",
            "Verified",
        );
        assert!(!single_job_is_terminal(&jobs, "test-job"));
    }

    #[test]
    fn recovery_failure_on_unwind_is_preserved_in_final_result() {
        let jobs = Arc::new(Mutex::new(HashMap::from([(
            "test-job".into(),
            password_test_job(None),
        )])));
        let recovery_jobs = Arc::clone(&jobs);
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = PasswordRecoveryGuard::new(move || {
                let message = "PASSWORD RECOVERY REQUIRED for 127.0.0.1";
                update_single_job_step(
                    &recovery_jobs,
                    "test-job",
                    "RestorePassword",
                    "Failed",
                    "Running",
                    message,
                );
                Err(message.into())
            });
            panic!("Simulated worker failure");
        }));
        let mut jobs = jobs.lock().unwrap();
        let job = jobs.get_mut("test-job").unwrap();
        finish_single_job(job, "Cancelled", "Generic cancellation message");
        assert_eq!(job.status, "Failed");
        assert!(job.message.contains("PASSWORD RECOVERY REQUIRED"));
    }

    #[test]
    fn parser_walks_nested_receiver_nodes_and_deduplicates() {
        let root: Value = serde_json::json!({
          "labs": [
            {
              "rack": [
                { "addr": "10.2.155.10", "user": "qa", "pw": "secret", "group": 1 },
                { "addr": "10.2.155.10", "user": "qa", "pw": "secret", "group": 1 },
                { "addr": "10.2.155.12", "user": "qa", "pw": "secret", "group": "2" },
                { "addr": "10.2.155.13", "user": "qa", "pw": "secret", "group": 9 }
              ]
            }
          ]
        });

        let mut found = Vec::new();
        walk_receiver_nodes(&root, &mut found);

        let mut unique: HashMap<String, ReceiverCredential> = HashMap::new();
        for rx in found {
            unique.entry(rx.addr.clone()).or_insert(rx);
        }

        assert_eq!(unique.len(), 2);
        assert!(unique.contains_key("10.2.155.10"));
        assert!(unique.contains_key("10.2.155.12"));
    }

    #[test]
    fn firmware_group_membership_comes_from_receiver_json_groups() {
        let credentials = vec![
            ReceiverCredential {
                addr: "10.2.155.45".to_string(),
                user: "qa".to_string(),
                pw: "secret".to_string(),
                group: 1,
            },
            ReceiverCredential {
                addr: "10.2.155.55".to_string(),
                user: "qa".to_string(),
                pw: "secret".to_string(),
                group: 2,
            },
        ];
        let configs = BTreeMap::from([
            (
                "Alpha".to_string(),
                FirmwareConfigMode {
                    receiver_models: BTreeMap::from([
                        ("10.2.155.45".to_string(), "argon".to_string()),
                        ("10.2.155.55".to_string(), "alloy".to_string()),
                    ]),
                    firmware_version: "V7.00".to_string(),
                    firmware_suffix: BTreeMap::new(),
                    logo: "ALPHA.png".to_string(),
                },
            ),
            (
                "Beta".to_string(),
                FirmwareConfigMode {
                    receiver_models: BTreeMap::new(),
                    firmware_version: "V5.83".to_string(),
                    firmware_suffix: BTreeMap::new(),
                    logo: "BETA.png".to_string(),
                },
            ),
        ]);

        let beta = firmware_receivers_for_group(&configs, "Beta", 1, credentials.clone());
        let alpha = firmware_receivers_for_group(&configs, "Alpha", 2, credentials);

        assert_eq!(beta.len(), 1);
        assert_eq!(beta[0].ip, "10.2.155.45");
        assert_eq!(beta[0].model, "argon");
        assert_eq!(alpha.len(), 1);
        assert_eq!(alpha[0].ip, "10.2.155.55");
        assert_eq!(alpha[0].model, "alloy");
    }

    #[test]
    fn firmware_target_uses_model_specific_suffix_before_default_override() {
        let config = FirmwareConfigMode {
            receiver_models: BTreeMap::new(),
            firmware_version: "V7.00".to_string(),
            firmware_suffix: BTreeMap::from([
                ("default".to_string(), "_titan_V700_034.timg".to_string()),
                ("tungsten".to_string(), "_V700_034.img".to_string()),
            ]),
            logo: "ALPHA.png".to_string(),
        };

        let (argon_file, _) =
            firmware_target_for_model(&config, "V7.00", "_custom_default.timg", "argon");
        let (tungsten_file, _) =
            firmware_target_for_model(&config, "V7.00", "_custom_default.timg", "tungsten");

        assert_eq!(argon_file, "argon_custom_default.timg");
        assert_eq!(tungsten_file, "tungsten_V700_034.img");
    }

    #[test]
    fn image_only_models_replace_default_timg_extension() {
        let config = FirmwareConfigMode {
            receiver_models: BTreeMap::new(),
            firmware_version: "V7.00".to_string(),
            firmware_suffix: BTreeMap::from([(
                "default".to_string(),
                "_titan_V700_034.timg".to_string(),
            )]),
            logo: "ALPHA.png".to_string(),
        };

        for model in ["silk", "stella", "yeager"] {
            let (file_name, _) = firmware_target_for_model(&config, "V7.00", "", model);
            assert_eq!(file_name, format!("{model}_titan_V700_034.img"));
        }
    }

    #[test]
    fn release_catalog_parser_keeps_only_direct_server_directories() {
        let base = firmware_release_base_url().expect("base URL should parse");
        let html = r#"
            <h3>Active Releases</h3>
            <ul>
              <li><a href="V6.70/index.html">V6.70 [ Alpha releases on main ]</a></li>
              <li><a href='V6.63/index.html'>V6.61/V6.63 [ Beta releases ]</a></li>
              <li><a href="nested/archive/index.html">Nested</a></li>
              <li><a href="https://example.com/V9.00/index.html">External</a></li>
            </ul>
            <h3>Released Code</h3>
            <ul><li><a href="V5.60/index.html">Historical</a></li></ul>
        "#;

        let releases = parse_firmware_release_links(&base, html).expect("catalog should parse");
        let ids: BTreeSet<_> = releases.into_iter().map(|release| release.id).collect();

        assert_eq!(
            ids,
            BTreeSet::from(["V6.63".to_string(), "V6.70".to_string()])
        );
    }

    #[test]
    fn build_catalog_parser_separates_alpha_and_beta_on_mixed_pages() {
        let index = firmware_release_index_url("V6.60").expect("release index should parse");
        let html = r#"
            <h1>V6.60 Alpha/Beta Releases</h1>
            <a href="../defects.css">Styles</a>
            <a href="0.43.html">Beta (66X branch) 6.60-0.43 [June 5, 2026]</a>
            <a href='0.40.html'>Alpha (main) 6.60-0.40 [May 15, 2026]</a>
            <a href='6.60.html'>Official Customer Release 6.60 [June 30, 2026]</a>
            <a href="index.html">Index</a>
            <a href="nested/0.39.html">Nested</a>
            <a href="https://example.com/0.39.html">External</a>
        "#;

        let alpha =
            parse_firmware_build_links(&index, html, "V6.60 [ Alpha and Beta releases ]", "Alpha")
                .expect("alpha catalog should parse");
        let beta =
            parse_firmware_build_links(&index, html, "V6.60 [ Alpha and Beta releases ]", "Beta")
                .expect("beta catalog should parse");

        assert_eq!(alpha[0].id, "0.40");
        assert!(alpha[0].name.contains("Alpha (main)"));
        assert_eq!(alpha.len(), 1);
        assert_eq!(beta[0].id, "0.43");
        assert!(beta[0].name.contains("Beta (66X branch)"));
        assert_eq!(beta.len(), 1);
    }

    #[test]
    fn build_catalog_parser_infers_channel_from_builds_on_neutral_page() {
        let index = firmware_release_index_url("V6.62").expect("release index should parse");
        let html = r#"
            <h1>V6.62</h1>
            <a href="6.62.html">Official Customer Release (on 660_only branch) V6.62 [August 28, 2026] for the Orbit project.</a>
            <a href="0.52.html">Quarantine Beta (on 660_only branch) V6.62-0.52 [August 20, 2026] for the Orbit project.</a>
            <a href="0.51.html">Quarantine Beta (on 660_only branch) V6.62-0.51 [August 04, 2026] for the Orbit project.</a>
            <a href="0.50.html">Quarantine Beta (on 660_only branch) V6.62-0.50 [July 29, 2026] for the Orbit project.</a>
        "#;

        let release_name = "V6.62";
        let beta = parse_firmware_build_links(&index, html, release_name, "Beta")
            .expect("beta catalog should parse");
        let alpha = parse_firmware_build_links(&index, html, release_name, "Alpha")
            .expect("alpha catalog should parse");

        assert_eq!(
            beta.iter()
                .map(|build| build.id.as_str())
                .collect::<Vec<_>>(),
            vec!["6.62", "0.52", "0.51", "0.50"]
        );
        assert!(beta[0].name.starts_with("Official Customer Release"));
        assert!(alpha.is_empty());
    }

    #[test]
    fn build_catalog_parser_inherits_channel_from_release_catalog_name() {
        let index = firmware_release_index_url("V6.62").expect("release index should parse");
        let html = r#"
            <h1>V6.62</h1>
            <a href="6.62.html">Official Customer Release V6.62 [August 28, 2026]</a>
        "#;
        let release_name = "V6.62 [ Orbit Project Official and Beta releases on 660 Only branch ]";

        let beta = parse_firmware_build_links(&index, html, release_name, "Beta")
            .expect("beta catalog should parse");
        let alpha = parse_firmware_build_links(&index, html, release_name, "Alpha")
            .expect("alpha catalog should parse");

        assert_eq!(beta.len(), 1);
        assert_eq!(beta[0].id, "6.62");
        assert!(alpha.is_empty());
    }

    #[test]
    fn build_catalog_parser_uses_single_channel_page_heading_for_unlabelled_builds() {
        let index = firmware_release_index_url("V5.83").expect("release index should parse");
        let html = r#"
            <h1>V5.83/6.43 Betas</h1>
            <a href="5.83.html">Official Customer Release 5.83/6.43 [September 2, 2025]</a>
            <a href="0.46.html">Quarantine Release 5.83/6.43 0.46 [August 22, 2025]</a>
        "#;

        let beta = parse_firmware_build_links(&index, html, "V5.83/6.43", "Beta")
            .expect("beta catalog should parse");
        let alpha = parse_firmware_build_links(&index, html, "V5.83/6.43", "Alpha")
            .expect("alpha catalog should parse");

        assert_eq!(beta.len(), 2);
        assert!(alpha.is_empty());
    }

    #[test]
    fn firmware_release_parser_reads_receiver_and_package_versions() {
        assert_eq!(firmware_release_version("6.70-0.26"), Some((6, 70)));
        assert_eq!(firmware_release_version("silk_V663_070.img"), Some((6, 63)));
        assert_eq!(
            firmware_release_version("argon_titan_V700_034.timg"),
            Some((7, 0))
        );
        assert_eq!(
            firmware_release_version("receiver_V4802_012.img"),
            Some((48, 2))
        );
    }

    #[test]
    fn downgrade_detection_compares_release_not_build_number() {
        assert!(is_firmware_downgrade(
            Some("6.70-0.26"),
            "silk_V663_070.img"
        ));
        assert!(!is_firmware_downgrade(
            Some("6.63-0.68"),
            "silk_V663_070.img"
        ));
        assert!(!is_firmware_downgrade(
            Some("6.63-0.70"),
            "silk_V670_026.img"
        ));
    }

    #[test]
    fn fast_install_fallback_recognizes_rebooted_uptime() {
        assert!(receiver_uptime_was_reset(Some(1_200), Some(45)));
        assert!(!receiver_uptime_was_reset(Some(1_200), Some(1_195)));
        assert!(!receiver_uptime_was_reset(None, Some(45)));
    }

    #[test]
    fn fast_image_packages_are_detected_by_extension() {
        assert!(is_fast_firmware_image("silk_V663_070.img"));
        assert!(is_fast_firmware_image("stella_V663_070.IMG"));
        assert!(!is_fast_firmware_image("argon_titan_V663_070.timg"));
    }

    #[test]
    fn fast_image_receiver_percent_uses_full_install_range() {
        assert_eq!(fast_image_install_percent(0.0), 15.0);
        assert_eq!(fast_image_install_percent(50.0), 43.5);
        assert_eq!(fast_image_install_percent(100.0), 72.0);
    }

    #[test]
    fn fast_image_workflow_reserves_progress_for_recovery() {
        assert_eq!(
            fast_image_workflow_progress("ReceiverRestart", "Running"),
            Some(82.0)
        );
        assert_eq!(
            fast_image_workflow_progress("ReceiverOnline", "Running"),
            Some(90.0)
        );
        assert_eq!(
            fast_image_workflow_progress("RestorePassword", "Completed"),
            Some(96.0)
        );
        assert_eq!(
            fast_image_workflow_progress("VerifyResult", "Running"),
            Some(98.0)
        );
    }

    #[test]
    fn timg_progress_keeps_three_receiver_stages() {
        assert!((firmware_overall_percent(1, 100.0) - 33.333_333).abs() < 0.001);
        assert_eq!(firmware_overall_percent(2, 50.0), 50.0);
        assert_eq!(firmware_overall_percent(3, 100.0), 100.0);
    }

    #[test]
    fn power_status_parser_reads_receiver_uptime() {
        let xml = r#"
            <data>
                <power>
                    <day>1</day>
                    <hour>2</hour>
                    <min>3</min>
                    <sec>4</sec>
                </power>
            </data>
        "#;
        let doc = roxmltree::Document::parse(xml).expect("power XML should parse");

        assert_eq!(get_uptime_seconds(&doc), Some(93_784));
    }

    #[test]
    fn release_package_parser_accepts_only_direct_firmware_files() {
        let release = firmware_release_url("V7.00").expect("release URL should parse");
        let html = r#"
            <a href="argon_titan_V700_034.timg">Argon</a>
            <a href='alloy_titan_V700_034.timg'>Alloy</a>
            <a href=krypton_titan_V700_034.timg>Krypton</a>
            <a href="tungsten_V700_034.img">Tungsten</a>
            <a href="nested/silk_titan_V700_034.timg">Nested</a>
            <a href="release-notes.txt">Notes</a>
            <a href="argon_titan_V700_034.timg?download=1">Query</a>
        "#;

        let packages = parse_firmware_package_links(&release, html).expect("packages should parse");
        let names: BTreeSet<_> = packages
            .into_iter()
            .map(|package| package.file_name)
            .collect();

        assert_eq!(
            names,
            BTreeSet::from([
                "alloy_titan_V700_034.timg".to_string(),
                "argon_titan_V700_034.timg".to_string(),
                "krypton_titan_V700_034.timg".to_string(),
                "tungsten_V700_034.img".to_string(),
            ])
        );
    }

    #[test]
    fn release_package_mapping_blocks_missing_and_ambiguous_models() {
        let packages = vec![
            FirmwareReleasePackage {
                file_name: "argon_titan_build_a.timg".to_string(),
                url: "http://server/argon_titan_build_a.timg".to_string(),
            },
            FirmwareReleasePackage {
                file_name: "argon_titan_build_b.timg".to_string(),
                url: "http://server/argon_titan_build_b.timg".to_string(),
            },
            FirmwareReleasePackage {
                file_name: "alloy_titan_build.timg".to_string(),
                url: "http://server/alloy_titan_build.timg".to_string(),
            },
        ];
        let models = vec!["argon".to_string(), "alloy".to_string(), "silk".to_string()];

        let matches = map_firmware_packages_to_models(&packages, &models, &BTreeMap::new());
        let statuses: BTreeMap<_, _> = matches
            .into_iter()
            .map(|result| (result.model, result.status))
            .collect();

        assert_eq!(statuses.get("argon").map(String::as_str), Some("Ambiguous"));
        assert_eq!(statuses.get("alloy").map(String::as_str), Some("Matched"));
        assert_eq!(statuses.get("silk").map(String::as_str), Some("Missing"));
    }

    #[test]
    fn release_package_mapping_uses_configured_file_extension() {
        let packages = vec![
            FirmwareReleasePackage {
                file_name: "argon_titan_V700_042.img".to_string(),
                url: "http://server/argon_titan_V700_042.img".to_string(),
            },
            FirmwareReleasePackage {
                file_name: "argon_titan_V700_042.timg".to_string(),
                url: "http://server/argon_titan_V700_042.timg".to_string(),
            },
        ];
        let matches = map_firmware_packages_to_models(
            &packages,
            &["argon".to_string()],
            &BTreeMap::from([("argon".to_string(), "timg".to_string())]),
        );

        assert_eq!(matches[0].status, "Matched");
        assert_eq!(
            matches[0].selected_file.as_deref(),
            Some("argon_titan_V700_042.timg")
        );
    }

    #[test]
    fn release_package_mapping_requires_img_for_image_only_models() {
        let packages = vec![
            FirmwareReleasePackage {
                file_name: "silk_titan_V700_042.timg".to_string(),
                url: "http://server/silk_titan_V700_042.timg".to_string(),
            },
            FirmwareReleasePackage {
                file_name: "silk_titan_V700_042.img".to_string(),
                url: "http://server/silk_titan_V700_042.img".to_string(),
            },
            FirmwareReleasePackage {
                file_name: "stella_titan_V700_042.img".to_string(),
                url: "http://server/stella_titan_V700_042.img".to_string(),
            },
            FirmwareReleasePackage {
                file_name: "yeager_titan_V700_042.img".to_string(),
                url: "http://server/yeager_titan_V700_042.img".to_string(),
            },
        ];
        let models = vec![
            "silk".to_string(),
            "stella".to_string(),
            "yeager".to_string(),
        ];

        let matches = map_firmware_packages_to_models(&packages, &models, &BTreeMap::new());

        assert!(matches.iter().all(|result| result.status == "Matched"));
        assert_eq!(
            matches
                .iter()
                .find(|result| result.model == "silk")
                .and_then(|result| result.selected_file.as_deref()),
            Some("silk_titan_V700_042.img")
        );
    }

    #[test]
    fn release_package_parser_maps_real_silk_img_name() {
        let release = firmware_release_url("V6.70").expect("release URL should parse");
        let html = r#"
            <a href="argon_titan_V670_026.timg">Argon Firmware</a>
            <a href="silk_V670_026.img">Silk Firmware [img format]</a>
            <a href="stella_V670_026.img">Stella Firmware [img format]</a>
            <a href="yeager_V670_026.img">Yeager Firmware [img format]</a>
        "#;
        let packages = parse_firmware_package_links(&release, html).expect("packages should parse");
        let matches = map_firmware_packages_to_models(
            &packages,
            &[
                "silk".to_string(),
                "stella".to_string(),
                "yeager".to_string(),
            ],
            &BTreeMap::new(),
        );

        assert!(matches.iter().all(|result| result.status == "Matched"));
        assert_eq!(
            matches
                .iter()
                .find(|result| result.model == "silk")
                .and_then(|result| result.selected_file.as_deref()),
            Some("silk_V670_026.img")
        );
    }

    #[test]
    fn image_only_model_validation_rejects_timg_packages() {
        assert!(validate_firmware_package_for_model("silk_build.img", "silk").is_ok());
        assert!(validate_firmware_package_for_model("stella_build.img", "stella").is_ok());
        assert!(validate_firmware_package_for_model("yeager_build.img", "yeager").is_ok());
        assert!(validate_firmware_package_for_model("silk_build.timg", "silk").is_err());
        assert!(validate_firmware_package_for_model("argon_build.timg", "argon").is_ok());
    }

    #[test]
    fn local_group_package_name_must_identify_its_receiver_model() {
        assert!(
            validate_firmware_package_name_for_model("argon_titan_V670_026.timg", "argon").is_ok()
        );
        assert!(validate_firmware_package_name_for_model("silk_V670_026.img", "silk").is_ok());
        assert!(
            validate_firmware_package_name_for_model("alloy_titan_V670_026.timg", "argon").is_err()
        );
        assert!(
            validate_firmware_package_name_for_model("firmware_V670_026.timg", "argon").is_err()
        );
    }

    #[test]
    fn release_target_rejects_path_traversal() {
        assert!(firmware_target_for_exact_package("../V7.00", "argon.timg").is_err());
        assert!(firmware_target_for_exact_package("V7.00", "../argon.timg").is_err());
        assert!(firmware_target_for_exact_package("V7.00", "notes.txt").is_err());
    }

    #[test]
    fn error_log_counter_uses_flags_and_ignores_warnings() {
        let xml = r#"
            <data>
              <errLog>
                <entry><err>7EE30106</err><flags>768</flags></entry>
                <entry><err>7000010F</err><flags>771</flags></entry>
                <entry><err>7EE30106</err><flags>768</flags></entry>
                <entry><err>7000010F</err><flags>771</flags></entry>
              </errLog>
            </data>
        "#;
        let doc = roxmltree::Document::parse(xml).expect("test XML should parse");

        assert_eq!(count_receiver_error_entries(&doc), 2);
    }

    #[test]
    fn clone_file_download_candidates_use_short_date_and_ip_name() {
        let today = today_clone_date();
        let requested_name = build_clone_file_base_name("10.2.155.107");
        let candidates =
            build_clone_file_download_candidates(&format!("{requested_name}.xml"), "IP107.xml");

        assert_eq!(requested_name, format!("{today}_IP107"));
        assert_eq!(
            candidates,
            vec![format!("{today}_IP107.xml"), "IP107.xml".to_string()]
        );
    }

    #[test]
    fn legacy_history_entries_remain_readable_after_operation_metadata_was_added() {
        let json = r#"{
            "jobId":"single-fw-1",
            "batchId":null,
            "ip":"10.2.155.107",
            "receiverName":"BD940-INS",
            "model":"argon",
            "targetFile":"argon_titan_V670_021.timg",
            "startedAt":"1",
            "completedAt":"2",
            "result":"Completed",
            "finalFirmwareVersion":"6.70-0.21",
            "cloneFileName":null,
            "cloneFilePath":null,
            "errors":[]
        }"#;

        let entry: FirmwareSingleUpdateHistoryEntry =
            serde_json::from_str(json).expect("legacy history should remain readable");

        assert_eq!(entry.operation_type, None);
        assert_eq!(entry.group_name, None);
        assert_eq!(entry.ip, "10.2.155.107");
    }

    #[test]
    fn update_safety_blocks_the_initial_post_restart_delay() {
        let settings = UpdateSafetySettings::default();
        let evaluation = evaluate_update_safety_window("10.2.155.11", 119, &settings);

        assert!(!evaluation.allowed);
        assert_eq!(evaluation.state, "PostRestartDelay");
        assert_eq!(evaluation.available_in_seconds, Some(1));
    }

    #[test]
    fn parallel_mode_uses_every_selected_receiver() {
        assert_eq!(receiver_concurrency(true, 16), 16);
        assert_eq!(receiver_concurrency(true, 25), 25);
        assert_eq!(receiver_concurrency(true, 80), 25);
        assert_eq!(receiver_concurrency(false, 16), 1);
    }

    #[test]
    fn multiple_batch_jobs_do_not_use_the_standalone_limit() {
        let (batch, batch_limit) =
            single_or_multiple_reservation("job-1", "192.0.2.10", Some("multiple-fw-123"));
        assert_eq!(batch.kind, FirmwareJobKind::Multiple);
        assert_eq!(batch_limit, usize::MAX);

        let (single, single_limit) = single_or_multiple_reservation("job-2", "192.0.2.11", None);
        assert_eq!(single.kind, FirmwareJobKind::Single);
        assert_eq!(single_limit, MAX_SINGLE_FIRMWARE_JOBS);
    }

    #[test]
    fn update_safety_allows_the_configured_middle_window() {
        let settings = UpdateSafetySettings::default();
        let start = evaluate_update_safety_window("10.2.155.11", 120, &settings);
        let end = evaluate_update_safety_window("10.2.155.11", 1_199, &settings);

        assert!(start.allowed);
        assert!(end.allowed);
        assert_eq!(start.state, "SafeWindow");
        assert_eq!(end.window_closes_in_seconds, Some(1));
    }

    #[test]
    fn update_safety_blocks_the_pre_restart_margin() {
        let settings = UpdateSafetySettings::default();
        let evaluation = evaluate_update_safety_window("10.2.155.11", 1_200, &settings);

        assert!(!evaluation.allowed);
        assert_eq!(evaluation.state, "ScheduledRestartMargin");
        assert_eq!(evaluation.available_in_seconds, Some(600));
    }

    #[test]
    fn update_safety_stops_restricting_after_the_restart_cycle() {
        let settings = UpdateSafetySettings::default();
        let evaluation = evaluate_update_safety_window("10.2.155.11", 1_800, &settings);

        assert!(evaluation.allowed);
        assert_eq!(evaluation.state, "CycleExceeded");
    }

    #[test]
    fn update_safety_settings_must_leave_a_valid_window() {
        let invalid = UpdateSafetySettings {
            post_restart_delay_minutes: 20,
            pre_restart_safety_margin_minutes: 10,
            restart_cycle_minutes: CONVERGENCE_RESTART_CYCLE_MINUTES,
        };

        assert!(validate_update_safety_settings(&invalid).is_err());
    }
}

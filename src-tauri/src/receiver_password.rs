use super::{clean_error, now_seconds, select_text, ReceiverCredential};
use reqwest::blocking::{Client, Response};
use reqwest::header::CACHE_CONTROL;
use reqwest::{redirect::Policy, StatusCode};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};
use std::{fmt, thread, time::Duration};

type OperationLocks = Mutex<HashMap<String, Arc<Mutex<()>>>>;
static OPERATION_LOCKS: OnceLock<OperationLocks> = OnceLock::new();

pub(super) fn operation_lock(ip: &str) -> Result<Arc<Mutex<()>>, String> {
    let mut locks = OPERATION_LOCKS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| "Receiver operation locks are unavailable.")?;
    Ok(Arc::clone(
        locks
            .entry(ip.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(()))),
    ))
}

const READ_ATTEMPTS: usize = 12;
const RETRY_DELAY: Duration = Duration::from_secs(5);

#[derive(Debug)]
enum PasswordError {
    Unavailable(String),
    Rejected,
    Locked,
    Unsafe(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FirmwareSecurityMode {
    PasswordProtected,
    Unprotected,
}

struct ReadyReceiver {
    security_mode: FirmwareSecurityMode,
    serial: Option<String>,
}

impl fmt::Display for PasswordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(message) | Self::Unsafe(message) => f.write_str(message),
            Self::Locked => f.write_str("Receiver access is locked or rate limited. Stop login attempts and check the receiver manually."),
            Self::Rejected => f.write_str(
                "Receiver rejected the credentials. No repeated login attempts were made.",
            ),
        }
    }
}

fn client() -> Result<Client, PasswordError> {
    Client::builder()
        .timeout(Duration::from_secs(8))
        .connect_timeout(Duration::from_secs(4))
        .redirect(Policy::none())
        .build()
        .map_err(|_| PasswordError::Unsafe("Cannot create password verification client.".into()))
}

fn transport_error(error: reqwest::Error) -> PasswordError {
    let retryable = error.is_timeout() || error.is_connect();
    let message = clean_error(error);
    if retryable {
        PasswordError::Unavailable(message)
    } else {
        PasswordError::Unsafe(message)
    }
}

fn authenticated_transport_error(error: reqwest::Error) -> PasswordError {
    PasswordError::Unsafe(format!(
        "The single login attempt could not be confirmed ({}). Login was not repeated to avoid a lockout.",
        clean_error(error)
    ))
}

fn check_response(response: Response) -> Result<Response, PasswordError> {
    match response.status() {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(PasswordError::Rejected),
        StatusCode::LOCKED | StatusCode::TOO_MANY_REQUESTS => Err(PasswordError::Locked),
        StatusCode::BAD_GATEWAY | StatusCode::SERVICE_UNAVAILABLE | StatusCode::GATEWAY_TIMEOUT => {
            Err(PasswordError::Unavailable(
                "Receiver is not ready after restart.".into(),
            ))
        }
        status if status.is_success() => Ok(response),
        status => Err(PasswordError::Unsafe(format!(
            "Unexpected password operation response: HTTP {}. Redirects are not accepted.",
            status.as_u16()
        ))),
    }
}

fn verify_once(rx: &ReceiverCredential, password: &str) -> Result<(), PasswordError> {
    let client = client()?;
    let serial = read_ready_serial(&client, rx)?;
    login_once(&client, rx, password, serial.as_deref())
}

fn read_ready_receiver(
    client: &Client,
    rx: &ReceiverCredential,
) -> Result<ReadyReceiver, PasswordError> {
    // Wait for the web interface without submitting credentials during reboot.
    // Public sysData is readiness information, never proof of authentication.
    let anonymous = client
        .get(format!("http://{}/xml/dynamic/merge.xml?sysData=", rx.addr))
        .header(CACHE_CONTROL, "no-store")
        .send()
        .map_err(transport_error)?;
    if anonymous.status() == StatusCode::UNAUTHORIZED {
        Ok(ReadyReceiver {
            security_mode: FirmwareSecurityMode::PasswordProtected,
            serial: None,
        })
    } else {
        let text = check_response(anonymous)?.text().map_err(transport_error)?;
        let doc = roxmltree::Document::parse(&text).map_err(|_| {
            PasswordError::Unsafe("Receiver readiness response is not valid XML.".into())
        })?;
        check_xml_failure(&doc)?;
        let sys = doc.root_element().children().find(|node| node.has_tag_name("sysData"))
            .ok_or_else(|| PasswordError::Unsafe("Receiver readiness response has no sysData. The password could not be verified.".into()))?;
        let field = |name| {
            sys.children()
                .find(|node| node.has_tag_name(name))
                .and_then(|node| node.text())
        };
        let user_security = field("UserSecurity").unwrap_or_default().trim();
        let security_mode = field("SecMode").unwrap_or_default().trim();
        if user_security.eq_ignore_ascii_case("FALSE") {
            return Ok(ReadyReceiver {
                security_mode: FirmwareSecurityMode::Unprotected,
                serial: field("serial").map(str::to_string),
            });
        }
        if user_security.eq_ignore_ascii_case("TRUE")
            && matches!(security_mode.to_ascii_lowercase().as_str(), "anon" | "full")
        {
            return Ok(ReadyReceiver {
                security_mode: FirmwareSecurityMode::PasswordProtected,
                serial: field("serial").map(str::to_string),
            });
        }
        Err(PasswordError::Unsafe(format!(
            "Receiver returned an unknown security state (UserSecurity={user_security}, SecMode={security_mode})."
        )))
    }
}

fn read_ready_serial(
    client: &Client,
    rx: &ReceiverCredential,
) -> Result<Option<String>, PasswordError> {
    let ready = read_ready_receiver(client, rx)?;
    if ready.security_mode == FirmwareSecurityMode::Unprotected {
        return Err(PasswordError::Unsafe(
            "Receiver security is disabled, so a password login cannot be verified.".into(),
        ));
    }
    Ok(ready.serial)
}

pub(super) fn check_firmware_access(
    rx: &ReceiverCredential,
    password: &str,
) -> Result<FirmwareSecurityMode, String> {
    let client = client()
        .map_err(|error| format!("Password security check failed for {}: {error}", rx.addr))?;
    let ready = read_ready_receiver(&client, rx)
        .map_err(|error| format!("Password security check failed for {}: {error}", rx.addr))?;
    if ready.security_mode == FirmwareSecurityMode::PasswordProtected {
        login_once(&client, rx, password, ready.serial.as_deref())
            .map_err(|error| format!("Password verification failed for {}: {error}", rx.addr))?;
    }
    Ok(ready.security_mode)
}

fn login_once(
    client: &Client,
    rx: &ReceiverCredential,
    password: &str,
    serial: Option<&str>,
) -> Result<(), PasswordError> {
    // Match the receiver's own xml/login.html form, inspected on BD940-INS
    // 6.63-0.73. FAIL/LOCKED are XML responses and may still have HTTP 200.
    let response = client
        .post(format!("http://{}/cgi-bin/login.xml", rx.addr))
        .header(CACHE_CONTROL, "no-store")
        .form(&[
            ("username", rx.user.clone()),
            ("password", password.to_string()),
            ("t", login_timestamp()),
        ])
        .send()
        .map_err(authenticated_transport_error)?;
    let text = check_response(response)
        .map_err(|error| match error {
            PasswordError::Unavailable(_) => PasswordError::Unsafe(
                "Receiver did not finish the login request. It was not repeated to avoid a lockout.".into(),
            ),
            other => other,
        })?
        .text()
        .map_err(authenticated_transport_error)?;
    parse_login_response(&text, serial)
}

fn login_timestamp() -> String {
    // The web form counts seconds from 1980-01-06, without leap-second adjustment.
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .saturating_sub(315_964_800)
        .to_string()
}

fn check_xml_failure(doc: &roxmltree::Document<'_>) -> Result<(), PasswordError> {
    if doc.descendants().any(|node| node.has_tag_name("LOCKED")) {
        return Err(PasswordError::Locked);
    }
    if doc.descendants().any(|node| node.has_tag_name("FAIL")) {
        return Err(PasswordError::Rejected);
    }
    Ok(())
}

fn parse_login_response(text: &str, expected_serial: Option<&str>) -> Result<(), PasswordError> {
    let doc = roxmltree::Document::parse(text).map_err(|_| {
        PasswordError::Unsafe(
            "Receiver login returned invalid XML. Password was not verified.".into(),
        )
    })?;
    check_xml_failure(&doc)?;
    let root = doc.root_element();
    let field = |name| {
        root.children()
            .find(|node| node.has_tag_name(name))
            .and_then(|node| node.text())
    };
    let cookie = field("cookie").unwrap_or_default();
    let serial = field("serial").unwrap_or_default();
    if !root.has_tag_name("data")
        || cookie.trim().is_empty()
        || serial.trim().is_empty()
        || expected_serial.is_some_and(|expected| serial != expected)
    {
        return Err(PasswordError::Unsafe(
            "Receiver login did not return its session confirmation. Public data or HTTP 200 alone does not verify a password.".into(),
        ));
    }
    // The login token proves acceptance; never persist or log it, and never reuse
    // a pre-change session when checking whether the new password really works.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    };
    use std::thread::JoinHandle;

    const CONFIGURED: &str = "configured&with+special=?#chars";
    const TEMPORARY: &str = "test-temporary";

    #[test]
    #[ignore = "Real receiver login: requires explicit operator approval and QA_APPROVED_LOGIN_CHECK_IP"]
    fn approved_receiver_login_only() {
        let ip =
            std::env::var("QA_APPROVED_LOGIN_CHECK_IP").expect("Explicit approval IP is required");
        ip.parse::<std::net::Ipv4Addr>()
            .expect("A single IPv4 address is required");
        let receivers = super::super::read_receivers(super::super::GROUPS_JSON_PATH)
            .expect("Cannot read receiver configuration");
        let rx = receivers
            .into_iter()
            .find(|rx| rx.addr == ip)
            .expect("Approved receiver is not in the configuration");
        // One verification only. No recovery, password write, upload, retry loop or scan.
        let result = verify(&rx, &rx.pw);
        assert!(result.is_ok(), "{}", result.unwrap_err());
        println!("Configured password accepted for {ip}. No password change or firmware upload was sent.");
    }

    #[derive(Clone, Copy, PartialEq)]
    enum Mode {
        Normal,
        Public,
        LoginPage,
        Redirect,
        RejectChange,
        FalseAck,
        LostReply,
        Locked,
        LockOnChange,
        ProtectedTelemetry,
        FullSecurity,
        XmlLocked,
        XmlLockOnChange,
        LoginUnavailable,
        LoginLostReply,
        ActivationCheckLost,
        DisabledSecurity,
        RestoredLoginUnavailable,
        RestoredLoginLostReply,
        RestoredLoginLocked,
        RestartAfterRestore,
    }

    struct MockState {
        mode: Mode,
        password: String,
        writes: usize,
        rejected_logins: usize,
        login_requests: usize,
        configured_login_requests: usize,
        restarting: usize,
        paths: Vec<String>,
    }

    struct ReceiverMock {
        rx: ReceiverCredential,
        state: Arc<Mutex<MockState>>,
        stop: Arc<AtomicBool>,
        worker: Option<JoinHandle<()>>,
    }

    fn authorization(password: &str) -> String {
        Client::new()
            .get("http://127.0.0.1")
            .basic_auth("test-user", Some(password))
            .build()
            .unwrap()
            .headers()[reqwest::header::AUTHORIZATION]
            .to_str()
            .unwrap()
            .to_string()
    }

    impl ReceiverMock {
        fn new(mode: Mode, password: &str, restarting: usize) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let rx = ReceiverCredential {
                addr: listener.local_addr().unwrap().to_string(),
                user: "test-user".into(),
                pw: CONFIGURED.into(),
                group: 1,
            };
            let state = Arc::new(Mutex::new(MockState {
                mode,
                password: password.into(),
                writes: 0,
                rejected_logins: 0,
                login_requests: 0,
                configured_login_requests: 0,
                restarting,
                paths: Vec::new(),
            }));
            let stop = Arc::new(AtomicBool::new(false));
            let worker_state = Arc::clone(&state);
            let worker_stop = Arc::clone(&stop);
            let worker = thread::spawn(move || {
                while !worker_stop.load(Ordering::SeqCst) {
                    let (mut stream, _) = match listener.accept() {
                        Ok(connection) => connection,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(1));
                            continue;
                        }
                        Err(error) => panic!("Mock accept failed: {error}"),
                    };
                    stream.set_nonblocking(false).unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let mut request = Vec::new();
                    let mut buffer = [0; 2048];
                    while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                        let size = stream.read(&mut buffer).unwrap();
                        if size == 0 {
                            break;
                        }
                        request.extend_from_slice(&buffer[..size]);
                    }
                    let header_end = request
                        .windows(4)
                        .position(|part| part == b"\r\n\r\n")
                        .unwrap()
                        + 4;
                    let head = String::from_utf8(request[..header_end].to_vec()).unwrap();
                    let content_length: usize = head
                        .lines()
                        .find_map(|line| {
                            line.split_once(':')
                                .filter(|(key, _)| key.eq_ignore_ascii_case("content-length"))
                                .map(|(_, value)| value.trim().parse().unwrap())
                        })
                        .unwrap_or_default();
                    while request.len() < header_end + content_length {
                        let size = stream.read(&mut buffer).unwrap();
                        assert!(size > 0, "Incomplete mock request");
                        request.extend_from_slice(&buffer[..size]);
                    }
                    let form = std::str::from_utf8(&request[header_end..]).unwrap();
                    let target = head
                        .lines()
                        .next()
                        .unwrap()
                        .split_whitespace()
                        .nth(1)
                        .unwrap();
                    let auth = head.lines().find_map(|line| {
                        line.split_once(':')
                            .filter(|(key, _)| key.eq_ignore_ascii_case("authorization"))
                            .map(|(_, value)| value.trim())
                    });
                    let mut state = worker_state.lock().unwrap();
                    let url = reqwest::Url::parse(&format!("http://localhost{target}")).unwrap();
                    state.paths.push(url.path().into());
                    let mut status = "200 OK";
                    let mut headers = "";
                    let public_data = "<data><sysData><UserSecurity>TRUE</UserSecurity><SecMode>Anon</SecMode><SecUser></SecUser><LoggedIn>0</LoggedIn><AuthTimeLeft>0</AuthTimeLeft><SecEditUser>0</SecEditUser><SecSetConfig>0</SecSetConfig><serial>TEST123</serial></sysData></data>";
                    let mut body = public_data;
                    if state.restarting > 0 {
                        state.restarting -= 1;
                        status = "503 Service Unavailable";
                    } else if url.path() == "/xml/dynamic/merge.xml" {
                        if state.mode == Mode::ProtectedTelemetry {
                            if auth != Some(authorization(&state.password).as_str()) {
                                state.rejected_logins += 1;
                                status = "401 Unauthorized";
                            }
                        } else if state.mode == Mode::FullSecurity {
                            assert!(auth.is_none());
                            status = "401 Unauthorized";
                        } else if state.mode == Mode::DisabledSecurity {
                            body = "<data><sysData><UserSecurity>FALSE</UserSecurity><SecMode>Off</SecMode></sysData></data>";
                        }
                    } else if state.mode == Mode::Redirect {
                        status = "302 Found";
                        headers = "Location: /login\r\n";
                    } else if url.path() == "/cgi-bin/login.xml" {
                        assert!(head.starts_with("POST "));
                        assert!(url.query().is_none());
                        assert!(auth.is_none());
                        assert!(!head.to_ascii_lowercase().contains("cookie:"));
                        assert!(head
                            .to_ascii_lowercase()
                            .contains("application/x-www-form-urlencoded"));
                        let form_url =
                            reqwest::Url::parse(&format!("http://localhost/?{form}")).unwrap();
                        let values: HashMap<_, _> = form_url.query_pairs().into_owned().collect();
                        assert_eq!(values["username"], "test-user");
                        let timestamp = values["t"].parse::<u64>().unwrap();
                        assert!(timestamp.abs_diff(login_timestamp().parse().unwrap()) <= 2);
                        state.login_requests += 1;
                        if values["password"] == CONFIGURED {
                            state.configured_login_requests += 1;
                        }
                        body = "<data><cookie>test-session-only</cookie><serial>TEST123</serial></data>";
                        match state.mode {
                            Mode::Public => body = public_data,
                            Mode::LoginPage => body = "<html><body>Please log in</body></html>",
                            Mode::Locked => status = "423 Locked",
                            Mode::XmlLocked => body = "<LOCKED>1</LOCKED>",
                            Mode::LoginUnavailable => status = "503 Service Unavailable",
                            Mode::LoginLostReply => continue,
                            Mode::RestoredLoginUnavailable if state.writes > 0 => {
                                status = "503 Service Unavailable"
                            }
                            Mode::RestoredLoginLostReply if state.writes > 0 => continue,
                            Mode::RestoredLoginLocked if state.writes > 0 => {
                                body = "<LOCKED>1</LOCKED>"
                            }
                            Mode::ActivationCheckLost if state.writes == 1 => {
                                state.mode = Mode::Normal;
                                continue;
                            }
                            _ if values["password"] != state.password => {
                                state.rejected_logins += 1;
                                body = "<FAIL>1</FAIL>";
                            }
                            _ => {}
                        }
                    } else if url.path() == "/cgi-bin/security_passwd.xml" {
                        state.writes += 1;
                        assert_eq!(auth, Some(authorization(&state.password).as_str()));
                        let values: std::collections::HashMap<_, _> =
                            url.query_pairs().into_owned().collect();
                        assert_eq!(values["CurrentPassword"], state.password);
                        assert_eq!(values["Username"], "test-user");
                        assert_eq!(values["NewPassword"], values["NewPassword2"]);
                        body = "<response><OK>1</OK></response>";
                        match state.mode {
                            Mode::LockOnChange => status = "423 Locked",
                            Mode::XmlLockOnChange => body = "<LOCKED>1</LOCKED>",
                            Mode::FalseAck => {}
                            Mode::RejectChange => body = "<response><OK>0</OK></response>",
                            _ => state.password = values["NewPassword"].clone(),
                        }
                        if state.mode == Mode::RestartAfterRestore {
                            state.restarting = 3;
                        }
                        if state.mode == Mode::LostReply {
                            continue;
                        }
                    } else {
                        panic!("Unexpected receiver request: {}", url.path());
                    }
                    write!(stream, "HTTP/1.1 {status}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
                }
            });
            Self {
                rx,
                state,
                stop,
                worker: Some(worker),
            }
        }
        fn restore(&self) -> Result<(), String> {
            restore_with_delay(&self.rx, TEMPORARY, Duration::ZERO)
        }
    }

    impl Drop for ReceiverMock {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            self.worker.take().unwrap().join().unwrap();
        }
    }

    #[test]
    fn public_receiver_checks_password_with_the_login_form_without_changing_it() {
        for mode in [Mode::Normal, Mode::FullSecurity] {
            let receiver = ReceiverMock::new(mode, CONFIGURED, 0);
            verify(&receiver.rx, CONFIGURED).unwrap();
            let state = receiver.state.lock().unwrap();
            assert_eq!(state.login_requests, 1);
            assert_eq!(state.writes, 0);
            assert_eq!(state.password, CONFIGURED);
            assert_eq!(
                state.paths,
                ["/xml/dynamic/merge.xml", "/cgi-bin/login.xml"]
            );
        }
    }

    #[test]
    fn login_xml_flags_override_http_success_and_session_fields() {
        assert!(matches!(
            parse_login_response("<FAIL>1</FAIL>", None),
            Err(PasswordError::Rejected)
        ));
        assert!(matches!(
            parse_login_response("<LOCKED/>", None),
            Err(PasswordError::Locked)
        ));
        assert!(matches!(parse_login_response("<data><FAIL>1</FAIL><LOCKED>1</LOCKED><cookie>fake</cookie><serial>TEST123</serial></data>", None), Err(PasswordError::Locked)));
        for xml in [
            "<data><OK>1</OK></data>",
            "<data><cookie></cookie><serial>TEST123</serial></data>",
            "<data><cookie>fake</cookie><serial>OTHER</serial></data>",
            "<html><cookie>fake</cookie><serial>TEST123</serial></html>",
            "<data><sysData><SecUser>test-user</SecUser><LoggedIn>1</LoggedIn></sysData></data>",
        ] {
            assert!(parse_login_response(xml, Some("TEST123")).is_err());
        }
        parse_login_response(
            "<data><cookie>test-session-only</cookie><serial>TEST123</serial></data>",
            Some("TEST123"),
        )
        .unwrap();
    }

    #[test]
    fn uncertain_login_response_is_not_retried() {
        for mode in [Mode::LoginUnavailable, Mode::LoginLostReply] {
            let receiver = ReceiverMock::new(mode, TEMPORARY, 0);
            assert!(receiver.restore().is_err());
            let state = receiver.state.lock().unwrap();
            assert_eq!(state.login_requests, 1);
            assert_eq!(state.writes, 0);
        }
    }

    #[test]
    fn initial_check_records_no_mutation_for_single_multiple_and_group_jobs() {
        use super::super::{
            check_initial_firmware_password, create_single_update_job_status, finish_single_job,
            password_recovery_error, FirmwarePlanReceiver,
        };
        for batch in [None, Some("multiple-fw-test"), Some("group-fw-test")] {
            for current in [CONFIGURED, "wrong-configured-password"] {
                let receiver = ReceiverMock::new(Mode::Normal, current, 0);
                let job = create_single_update_job_status(
                    "test-job",
                    &FirmwarePlanReceiver {
                        credential: receiver.rx.clone(),
                        model: "BD940-INS".into(),
                        target_file: "argon_titan_V663_073.timg".into(),
                        target_url: "local://test".into(),
                        local_package_path: None,
                    },
                    batch.map(str::to_string),
                    "Test",
                );
                let jobs = Arc::new(Mutex::new(HashMap::from([("test-job".into(), job)])));
                let result = check_initial_firmware_password(&jobs, "test-job", &receiver.rx);
                if current == CONFIGURED {
                    assert_eq!(
                        result.as_ref().ok(),
                        Some(&FirmwareSecurityMode::PasswordProtected)
                    );
                }
                let mut jobs = jobs.lock().unwrap();
                let job = jobs.get_mut("test-job").unwrap();
                assert_eq!(result.is_ok(), current == CONFIGURED);
                if let Err(error) = result {
                    assert!(error
                        .contains("No password change or firmware upload was sent by this job"));
                    finish_single_job(job, "Failed", &error);
                    assert!(password_recovery_error(job).is_none());
                    assert!(job.message.contains("before the password change"));
                }
                assert!(job.steps.iter().any(|step| step.state == "CheckPassword"));
                assert!(!job.steps.iter().any(|step| step.state == "ChangePassword"));
                let state = receiver.state.lock().unwrap();
                assert_eq!(state.writes, 0);
                assert_eq!(state.login_requests, 1);
                assert_eq!(state.password, current);
            }
        }
    }

    #[test]
    fn disabled_security_skips_login_and_password_mutation_before_firmware() {
        use super::super::{
            check_initial_firmware_password, create_single_update_job_status, FirmwarePlanReceiver,
        };

        let receiver = ReceiverMock::new(Mode::DisabledSecurity, CONFIGURED, 0);
        let job = create_single_update_job_status(
            "test-job",
            &FirmwarePlanReceiver {
                credential: receiver.rx.clone(),
                model: "BD940-INS".into(),
                target_file: "argon_titan_V663_073.timg".into(),
                target_url: "local://test".into(),
                local_package_path: None,
            },
            Some("group-fw-test".into()),
            "Test",
        );
        let jobs = Arc::new(Mutex::new(HashMap::from([("test-job".into(), job)])));

        let mode = check_initial_firmware_password(&jobs, "test-job", &receiver.rx).unwrap();

        assert_eq!(mode, FirmwareSecurityMode::Unprotected);
        let jobs = jobs.lock().unwrap();
        let step = jobs["test-job"]
            .steps
            .iter()
            .rev()
            .find(|step| step.state == "CheckPassword")
            .unwrap();
        assert_eq!(step.status, "Completed");
        assert!(step.message.contains("security is disabled"));
        let state = receiver.state.lock().unwrap();
        assert_eq!(state.login_requests, 0);
        assert_eq!(state.writes, 0);
    }

    #[test]
    fn public_receiver_can_activate_and_restore_password_with_fresh_logins() {
        let receiver = ReceiverMock::new(Mode::Normal, CONFIGURED, 0);
        verify(&receiver.rx, CONFIGURED).unwrap();
        assert!(
            super::super::activate_temporary_firmware_password(&receiver.rx, TEMPORARY).unwrap()
        );
        assert_eq!(receiver.state.lock().unwrap().password, TEMPORARY);
        receiver.restore().unwrap();
        let state = receiver.state.lock().unwrap();
        assert_eq!(state.password, CONFIGURED);
        assert_eq!(state.writes, 2);
        assert_eq!(state.login_requests, 4);
        assert_eq!(state.rejected_logins, 0);
    }

    #[test]
    fn lost_activation_check_is_reported_after_change_and_recovery_can_finish() {
        let receiver = ReceiverMock::new(Mode::ActivationCheckLost, CONFIGURED, 0);
        verify(&receiver.rx, CONFIGURED).unwrap();
        let error = super::super::activate_temporary_firmware_password(&receiver.rx, TEMPORARY)
            .unwrap_err();
        assert!(error.contains("temporary password change was acknowledged"));
        assert_eq!(receiver.state.lock().unwrap().password, TEMPORARY);
        receiver.restore().unwrap();
        let state = receiver.state.lock().unwrap();
        assert_eq!(state.password, CONFIGURED);
        assert_eq!(state.writes, 2);
        assert_eq!(state.rejected_logins, 0);
    }

    #[test]
    fn matching_temporary_and_configured_password_requires_login_but_no_write() {
        let receiver = ReceiverMock::new(Mode::Normal, CONFIGURED, 0);
        restore_with_delay(&receiver.rx, CONFIGURED, Duration::ZERO).unwrap();
        let state = receiver.state.lock().unwrap();
        assert_eq!(state.writes, 0);
        assert_eq!(state.login_requests, 1);
    }

    #[test]
    fn public_telemetry_does_not_skip_password_restore() {
        let receiver = ReceiverMock::new(Mode::Normal, TEMPORARY, 0);
        // This was the old verifier: wrong credentials still get public XML with HTTP 200.
        assert!(super::super::get_xml_text_with_password(
            &Client::new(),
            &receiver.rx,
            CONFIGURED,
            &format!("http://{}/xml/dynamic/merge.xml?sysData=", receiver.rx.addr)
        )
        .is_ok());
        receiver.restore().unwrap();
        let state = receiver.state.lock().unwrap();
        assert_eq!(state.password, CONFIGURED);
        assert_eq!(state.writes, 1);
        assert_eq!(state.rejected_logins, 0);
    }

    #[test]
    fn waits_for_reboot_without_sending_wrong_passwords() {
        let receiver = ReceiverMock::new(Mode::Normal, TEMPORARY, 3);
        receiver.restore().unwrap();
        let state = receiver.state.lock().unwrap();
        assert_eq!(state.password, CONFIGURED);
        assert_eq!(state.rejected_logins, 0);
    }

    #[test]
    fn firmware_monitor_does_not_fall_back_to_another_password() {
        let receiver = ReceiverMock::new(Mode::ProtectedTelemetry, CONFIGURED, 0);
        let result =
            super::super::receiver_firmware_live_probe(&Client::new(), &receiver.rx, TEMPORARY);
        assert!(result
            .as_ref()
            .err()
            .is_some_and(|error| super::super::is_receiver_authentication_error(error)));
        let state = receiver.state.lock().unwrap();
        assert_eq!(state.rejected_logins, 1);
        assert_eq!(state.paths.len(), 1);
    }

    #[test]
    fn already_restored_password_is_verified_without_a_write() {
        let receiver = ReceiverMock::new(Mode::Normal, CONFIGURED, 0);
        receiver.restore().unwrap();
        let state = receiver.state.lock().unwrap();
        assert_eq!(state.writes, 0);
        assert_eq!(state.rejected_logins, 1);
    }

    #[test]
    fn rejected_credentials_are_not_retried_twelve_times() {
        let receiver = ReceiverMock::new(Mode::Normal, "unknown", 0);
        assert!(receiver.restore().is_err());
        let state = receiver.state.lock().unwrap();
        assert_eq!(state.rejected_logins, 2);
        assert_eq!(state.writes, 0);
    }

    #[test]
    fn false_acknowledgement_and_rejection_are_not_success() {
        for mode in [Mode::FalseAck, Mode::RejectChange] {
            let receiver = ReceiverMock::new(mode, TEMPORARY, 0);
            assert!(receiver.restore().is_err());
            let state = receiver.state.lock().unwrap();
            assert_eq!(state.writes, 1);
            assert_eq!(state.rejected_logins, 1);
            assert_eq!(state.password, TEMPORARY);
        }
    }

    #[test]
    fn lost_write_response_is_checked_without_repeating_the_write() {
        let receiver = ReceiverMock::new(Mode::LostReply, TEMPORARY, 0);
        receiver.restore().unwrap();
        assert_eq!(receiver.state.lock().unwrap().writes, 1);
    }

    #[test]
    fn lockout_during_password_change_stops_all_further_login_attempts() {
        for mode in [Mode::LockOnChange, Mode::XmlLockOnChange] {
            let receiver = ReceiverMock::new(mode, TEMPORARY, 0);
            assert!(receiver.restore().unwrap_err().contains("locked"));
            let state = receiver.state.lock().unwrap();
            assert_eq!(state.writes, 1);
            assert_eq!(state.paths.len(), 3);
            assert_eq!(state.login_requests, 1);
            assert_eq!(state.rejected_logins, 0);
        }
    }

    #[test]
    fn anonymous_endpoint_login_page_redirect_and_lockout_fail_closed() {
        for mode in [
            Mode::Public,
            Mode::LoginPage,
            Mode::Redirect,
            Mode::Locked,
            Mode::XmlLocked,
            Mode::DisabledSecurity,
        ] {
            let receiver = ReceiverMock::new(mode, TEMPORARY, 0);
            assert!(receiver.restore().is_err());
            let state = receiver.state.lock().unwrap();
            assert_eq!(state.writes, 0);
            assert_eq!(state.rejected_logins, 0);
            assert!(!state.paths.iter().any(|path| path == "/login"));
        }
    }

    #[test]
    fn retry_limit_applies_only_to_unavailable_reads() {
        let receiver = ReceiverMock::new(Mode::Normal, TEMPORARY, READ_ATTEMPTS + 1);
        assert!(receiver.restore().is_err());
        let state = receiver.state.lock().unwrap();
        assert_eq!(state.paths.len(), READ_ATTEMPTS);
        assert!(state
            .paths
            .iter()
            .all(|path| path == "/xml/dynamic/merge.xml"));
        assert_eq!(state.login_requests, 0);
        assert_eq!(state.writes, 0);
    }

    #[test]
    fn restored_password_has_exactly_one_confirmation_attempt() {
        for mode in [
            Mode::Normal,
            Mode::FalseAck,
            Mode::RestoredLoginUnavailable,
            Mode::RestoredLoginLostReply,
            Mode::RestoredLoginLocked,
            Mode::RestartAfterRestore,
        ] {
            let receiver = ReceiverMock::new(mode, TEMPORARY, 0);
            let result = receiver.restore();
            assert_eq!(
                result.is_ok(),
                matches!(mode, Mode::Normal | Mode::RestartAfterRestore)
            );
            let state = receiver.state.lock().unwrap();
            assert_eq!(state.writes, 1);
            assert_eq!(state.configured_login_requests, 1);
            // The other login precedes the write and checks the expected temporary credential.
            assert_eq!(state.login_requests, 2);
        }
    }

    #[test]
    fn login_connection_failure_is_terminal_not_retryable() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let rx = ReceiverCredential {
            addr: listener.local_addr().unwrap().to_string(),
            user: "test-user".into(),
            pw: CONFIGURED.into(),
            group: 1,
        };
        drop(listener);
        let result = login_once(&client().unwrap(), &rx, CONFIGURED, None);
        assert!(matches!(result, Err(PasswordError::Unsafe(_))));
    }

    #[test]
    fn scan_and_password_change_share_a_per_receiver_lock() {
        let firmware = operation_lock("192.0.2.1").unwrap();
        let scan = operation_lock("192.0.2.1").unwrap();
        let other = operation_lock("192.0.2.2").unwrap();
        let guard = firmware.lock().unwrap();
        assert!(scan.try_lock().is_err());
        assert!(other.try_lock().is_ok());
        drop(guard);
        assert!(scan.try_lock().is_ok());
    }
}

fn wait_until_ready(
    client: &Client,
    rx: &ReceiverCredential,
    delay: Duration,
) -> Result<Option<String>, PasswordError> {
    for attempt in 1..=READ_ATTEMPTS {
        match read_ready_serial(client, rx) {
            Err(PasswordError::Unavailable(_)) if attempt < READ_ATTEMPTS => thread::sleep(delay),
            result => return result,
        }
    }
    unreachable!()
}

fn verify_when_ready(
    rx: &ReceiverCredential,
    password: &str,
    delay: Duration,
) -> Result<(), PasswordError> {
    let client = client()?;
    let serial = wait_until_ready(&client, rx, delay)?;
    // Only anonymous readiness reads may repeat. The credential-bearing request
    // is deliberately outside the retry loop, including on connection errors.
    login_once(&client, rx, password, serial.as_deref())
}

pub(super) fn verify(rx: &ReceiverCredential, password: &str) -> Result<(), String> {
    verify_once(rx, password)
        .map_err(|error| format!("Password verification failed for {}: {error}", rx.addr))
}

pub(super) fn change(rx: &ReceiverCredential, current: &str, new: &str) -> Result<(), String> {
    change_once(rx, current, new)
        .map_err(|error| format!("Password change was not confirmed for {}: {error}", rx.addr))
}

fn change_once(rx: &ReceiverCredential, current: &str, new: &str) -> Result<(), PasswordError> {
    let response = client()?
        .get(format!("http://{}/cgi-bin/security_passwd.xml", rx.addr))
        .basic_auth(&rx.user, Some(current))
        .header(CACHE_CONTROL, "no-store")
        .query(&[
            ("t", now_seconds()),
            ("CurrentPassword", current.to_string()),
            ("Username", rx.user.clone()),
            ("NewPassword", new.to_string()),
            ("NewPassword2", new.to_string()),
        ])
        .send()
        .map_err(transport_error)?;
    let text = check_response(response)?.text().map_err(transport_error)?;
    let doc = roxmltree::Document::parse(&text)
        .map_err(|_| PasswordError::Unsafe("Invalid password change response.".into()))?;
    check_xml_failure(&doc)?;
    if select_text(&doc, "OK").as_deref() != Some("1") {
        return Err(PasswordError::Unsafe(
            "Receiver did not confirm the password change.".into(),
        ));
    }
    Ok(())
}

pub(super) fn restore(rx: &ReceiverCredential, temporary: &str) -> Result<(), String> {
    restore_with_delay(rx, temporary, RETRY_DELAY)
        .map_err(|error| format!("PASSWORD RECOVERY REQUIRED for {}. {error} Stop other scripts using this receiver and check its password manually before another update.", rx.addr))
}

fn restore_with_delay(
    rx: &ReceiverCredential,
    temporary: &str,
    delay: Duration,
) -> Result<(), String> {
    if temporary == rx.pw {
        return verify_when_ready(rx, &rx.pw, delay).map_err(|error| error.to_string());
    }
    // The temporary password is the expected current credential. Do not keep
    // trying the configured (currently wrong) password while waiting for reboot.
    match verify_when_ready(rx, temporary, delay) {
        Ok(()) => {}
        Err(PasswordError::Rejected) => {
            return verify_when_ready(rx, &rx.pw, delay).map_err(|error| error.to_string());
        }
        Err(error) => return Err(error.to_string()),
    }

    // A timeout can mean the write succeeded but its response was lost. Never
    // repeat this write blindly; check the desired password in a fresh request.
    let change_result = change_once(rx, temporary, &rx.pw);
    if matches!(change_result, Err(PasswordError::Locked)) {
        return Err(PasswordError::Locked.to_string());
    }
    match verify_when_ready(rx, &rx.pw, delay) {
        Ok(()) => Ok(()),
        Err(error) => Err(match change_result {
            Ok(()) => format!("Receiver acknowledged the change, but the configured password could not be verified: {error}"),
            Err(change_error) => format!("{change_error} Configured password verification also failed: {error}"),
        }),
    }
}

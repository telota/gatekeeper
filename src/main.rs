use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration};

use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use askama::Template;
use rand::{Rng, RngCore};
use hyper_util::rt::TokioIo;
use hyper_util::server::conn::auto;

pub mod config;
pub mod crypto;
pub mod theme;

use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug)]
pub struct DomainMetrics {
    pub honeypot_hits: AtomicU64,
    pub blocks_no_cookie: AtomicU64,
    pub blocks_invalid_cookie: AtomicU64,
    pub cookie_accepted: AtomicU64,
    pub verification_successful: AtomicU64,
    pub verification_failed: AtomicU64,
    pub good_bot_passes: AtomicU64,
}

impl Default for DomainMetrics {
    fn default() -> Self {
        Self {
            honeypot_hits: AtomicU64::new(0),
            blocks_no_cookie: AtomicU64::new(0),
            blocks_invalid_cookie: AtomicU64::new(0),
            cookie_accepted: AtomicU64::new(0),
            verification_successful: AtomicU64::new(0),
            verification_failed: AtomicU64::new(0),
            good_bot_passes: AtomicU64::new(0),
        }
    }
}

impl serde::Serialize for DomainMetrics {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("DomainMetrics", 7)?;
        state.serialize_field("honeypot_hits", &self.honeypot_hits.load(Ordering::Relaxed))?;
        state.serialize_field("blocks_no_cookie", &self.blocks_no_cookie.load(Ordering::Relaxed))?;
        state.serialize_field("blocks_invalid_cookie", &self.blocks_invalid_cookie.load(Ordering::Relaxed))?;
        state.serialize_field("cookie_accepted", &self.cookie_accepted.load(Ordering::Relaxed))?;
        state.serialize_field("verification_successful", &self.verification_successful.load(Ordering::Relaxed))?;
        state.serialize_field("verification_failed", &self.verification_failed.load(Ordering::Relaxed))?;
        state.serialize_field("good_bot_passes", &self.good_bot_passes.load(Ordering::Relaxed))?;
        state.end()
    }
}

/// Application state shared across Axum handlers.
struct AppState {
    config: config::Config,
    whitelist: std::sync::RwLock<std::collections::HashMap<String, Vec<(std::net::IpAddr, u8)>>>,
    theme: std::sync::RwLock<theme::Theme>,
    start_time: u64,
    metrics: std::sync::RwLock<std::collections::HashMap<String, Arc<DomainMetrics>>>,
}

impl AppState {
    fn increment_metric<R>(&self, host: String, select: impl FnOnce(&DomainMetrics) -> R) {
        if let Ok(guard) = self.metrics.read() {
            if let Some(metrics) = guard.get(&host) {
                let _ = select(metrics);
                return;
            }
        }
        if let Ok(mut guard) = self.metrics.write() {
            let metrics = guard.entry(host).or_insert_with(|| Arc::new(DomainMetrics::default())).clone();
            let _ = select(&metrics);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 1. CLI Argument Parsing
    let args: Vec<String> = env::args().collect();
    let config_path = "config.toml";

    if args.len() > 1 && (args[1] == "KeyGenerate" || args[1] == "key-generate") {
        // Run KeyGenerate CLI command
        if !Path::new(config_path).exists() {
            eprintln!("Error: config.toml not found in the current directory.");
            std::process::exit(1);
        }

        let new_key = if args.len() > 2 {
            args[2].clone()
        } else {
            config::generate_random_key()
        };

        config::update_app_key(config_path, &new_key)?;
        println!("Successfully set/generated app_key in config.toml to: {}", new_key);
        return Ok(());
    }

    // 2. Normal App Startup: Load configuration
    let config = match config::load_config(config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("FATAL CONFIGURATION ERROR: {}", e);
            std::process::exit(1);
        }
    };

    let whitelist_rules = if let Some(ref path) = config.gatekeeper.whitelist_path {
        match fs::read_to_string(path) {
            Ok(content) => {
                let rules = parse_whitelist_file(&content);
                println!("Loaded {} whitelist bots from {}", rules.len(), path);
                rules
            }
            Err(e) => {
                println!("Notice: Could not read whitelist file at {} ({}). Proceeding with empty whitelist.", path, e);
                std::collections::HashMap::new()
            }
        }
    } else {
        println!("Notice: No whitelist path configured. Proceeding with empty whitelist.");
        std::collections::HashMap::new()
    };

    let theme_path = config.gatekeeper.theme_path.as_deref().unwrap_or("theme.toml");
    let initial_theme = theme::Theme::load_or_default(Some(Path::new(theme_path)));

    let shared_state = Arc::new(AppState {
        config: config.clone(),
        whitelist: std::sync::RwLock::new(whitelist_rules),
        theme: std::sync::RwLock::new(initial_theme),
        start_time: get_current_timestamp(),
        metrics: std::sync::RwLock::new(std::collections::HashMap::new()),
    });

    // 4. Build Axum Router
    let app = Router::new()
        .route("/auth/verify", get(get_verify))
        .route("/auth/verify", post(post_verify))
        .route("/auth/check", get(get_check))
        .route("/auth/metrics", get(get_metrics))
        .route("/honeypot", get(get_honeypot))
        .layer(axum::extract::DefaultBodyLimit::max(1024 * 16))
        .with_state(shared_state.clone());

    // 5. Unix Domain Socket Binding & Listening
    let socket_path = &config.gatekeeper.socket_path;

    // Clean up existing socket file if it exists
    if Path::new(socket_path).exists() {
        println!("Removing existing socket file at {}", socket_path);
        let _ = fs::remove_file(socket_path);
    }

    // Ensure parent directories of the socket path exist
    if let Some(parent) = Path::new(socket_path).parent() {
        fs::create_dir_all(parent)?;
    }

    let listener = tokio::net::UnixListener::bind(socket_path)?;

    // Set open permissions for the socket so Nginx proxy can access it
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(socket_path)?.permissions();
        perms.set_mode(0o666);
        fs::set_permissions(socket_path, perms)?;
    }

    println!("Gatekeeper service listening on Unix socket: {}", socket_path);

    // 5.5 Spawn SIGHUP signal handler for hot-reloading whitelist and theme
    let signal_state = shared_state.clone();
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        if let Ok(mut stream) = signal(SignalKind::hangup()) {
            tokio::spawn(async move {
                while stream.recv().await.is_some() {
                    println!("SIGHUP received. Reloading whitelist and theme...");
                    if let Some(ref path) = signal_state.config.gatekeeper.whitelist_path {
                        match fs::read_to_string(path) {
                            Ok(content) => {
                                let new_rules = parse_whitelist_file(&content);
                                match signal_state.whitelist.write() {
                                    Ok(mut guard) => {
                                        *guard = new_rules;
                                        println!("Successfully reloaded {} whitelist rules.", guard.len());
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to acquire write lock for whitelist: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to read whitelist file at {}: {}", path, e);
                            }
                        }
                    } else {
                        println!("No whitelist path configured, skipping reload.");
                    }

                    // Hot-reload theme
                    let theme_path = signal_state.config.gatekeeper.theme_path.as_deref().unwrap_or("theme.toml");
                    let new_theme = theme::Theme::load_or_default(Some(Path::new(theme_path)));
                    match signal_state.theme.write() {
                        Ok(mut guard) => {
                            *guard = new_theme;
                            println!("Successfully reloaded theme from {}", theme_path);
                        }
                        Err(e) => {
                            eprintln!("Failed to acquire write lock for theme: {}", e);
                        }
                    }
                }
            });
        }
    }

    // 6. Serve Axum app over Unix Domain Socket using Hyper
    loop {
        let (stream, _addr) = match listener.accept().await {
            Ok(val) => val,
            Err(e) => {
                eprintln!("Error accepting Unix socket connection: {}", e);
                sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        let io = TokioIo::new(stream);
        let app = app.clone();
        
        tokio::spawn(async move {
            let service = hyper_util::service::TowerToHyperService::new(app);
            if let Err(_err) = auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                .serve_connection(io, service)
                .await
            {
                // Silence common connection reset errors
            }
        });
    }
}

/// Normalizes IPv6-mapped IPv4 addresses to standard IPv4.
fn normalize_ip(ip: std::net::IpAddr) -> std::net::IpAddr {
    match ip {
        std::net::IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4() {
                std::net::IpAddr::V4(v4)
            } else {
                std::net::IpAddr::V6(v6)
            }
        }
        v4 => v4,
    }
}

/// Checks if an IPv4 address is within the given subnet prefix.
fn ip_in_cidr_v4(ip: &std::net::Ipv4Addr, cidr_ip: &std::net::Ipv4Addr, prefix: u8) -> bool {
    if prefix > 32 {
        return false;
    }
    let ip_u32 = u32::from_be_bytes(ip.octets());
    let cidr_u32 = u32::from_be_bytes(cidr_ip.octets());
    if prefix == 0 {
        true
    } else {
        let mask = !0u32 << (32 - prefix);
        (ip_u32 & mask) == (cidr_u32 & mask)
    }
}

/// Checks if an IPv6 address is within the given subnet prefix.
fn ip_in_cidr_v6(ip: &std::net::Ipv6Addr, cidr_ip: &std::net::Ipv6Addr, prefix: u8) -> bool {
    if prefix > 128 {
        return false;
    }
    let ip_u128 = u128::from_be_bytes(ip.octets());
    let cidr_u128 = u128::from_be_bytes(cidr_ip.octets());
    if prefix == 0 {
        true
    } else {
        let mask = !0u128 << (128 - prefix);
        (ip_u128 & mask) == (cidr_u128 & mask)
    }
}

/// Performs a CIDR match for any IP address family.
fn ip_in_cidr(ip: &std::net::IpAddr, cidr_ip: &std::net::IpAddr, prefix: u8) -> bool {
    let norm_ip = normalize_ip(*ip);
    let norm_cidr = normalize_ip(*cidr_ip);
    match (norm_ip, norm_cidr) {
        (std::net::IpAddr::V4(ip_v4), std::net::IpAddr::V4(cidr_v4)) => {
            ip_in_cidr_v4(&ip_v4, &cidr_v4, prefix)
        }
        (std::net::IpAddr::V6(ip_v6), std::net::IpAddr::V6(cidr_v6)) => {
            ip_in_cidr_v6(&ip_v6, &cidr_v6, prefix)
        }
        _ => false,
    }
}

/// Parses a single CIDR string into an IP address and a prefix.
fn parse_cidr(cidr_str: &str) -> Option<(std::net::IpAddr, u8)> {
    let parts: Vec<&str> = cidr_str.split('/').collect();
    if parts.is_empty() || parts.len() > 2 {
        return None;
    }
    let ip_str = parts[0];
    let ip: std::net::IpAddr = ip_str.parse().ok()?;
    let prefix = if parts.len() == 2 {
        parts[1].parse::<u8>().ok()?
    } else {
        match ip {
            std::net::IpAddr::V4(_) => 32,
            std::net::IpAddr::V6(_) => 128,
        }
    };

    match ip {
        std::net::IpAddr::V4(_) if prefix > 32 => return None,
        std::net::IpAddr::V6(_) if prefix > 128 => return None,
        _ => {}
    }

    Some((normalize_ip(ip), prefix))
}

/// Parses the contents of a whitelist file.
fn parse_whitelist_file(content: &str) -> std::collections::HashMap<String, Vec<(std::net::IpAddr, u8)>> {
    let mut map = std::collections::HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 2 {
            continue;
        }
        let ua_pattern = parts[0].to_lowercase();
        if let Some(cidr_data) = parse_cidr(parts[1]) {
            map.entry(ua_pattern)
                .or_insert_with(Vec::new)
                .push(cidr_data);
        }
    }
    map
}

/// Checks if the request comes from a whitelisted bot based on User-Agent and IP.
fn check_whitelist(state: &AppState, headers: &HeaderMap) -> bool {
    let ua = crypto::get_header_value(headers, "user-agent").to_lowercase();
    let ip_str = crypto::get_header_value(headers, "x-original-ip");
    let ip: std::net::IpAddr = match ip_str.parse() {
        Ok(parsed) => parsed,
        Err(_) => return false,
    };

    let whitelist = match state.whitelist.read() {
        Ok(guard) => guard,
        Err(_) => return false,
    };

    for (pattern, cidrs) in whitelist.iter() {
        if ua.contains(pattern) {
            for (cidr_ip, prefix) in cidrs {
                if ip_in_cidr(&ip, cidr_ip, *prefix) {
                    return true;
                }
            }
            return false;
        }
    }
    false
}

#[derive(serde::Serialize)]
struct MetricsResponse {
    start_time: u64,
    domains: std::collections::HashMap<String, Arc<DomainMetrics>>,
}

/// GET /auth/metrics
/// Endpoint for fetching current daemon metrics and start time.
async fn get_metrics(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let domains = match state.metrics.read() {
        Ok(guard) => guard.clone(),
        Err(_) => std::collections::HashMap::new(),
    };
    let response = MetricsResponse {
        start_time: state.start_time,
        domains,
    };
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], serde_json::to_string(&response).unwrap_or_default())
}

/// Helper to get current Unix timestamp in seconds.
fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Encodes raw strings to RFC 3986 percent-encoding.
/// Essential for forwarding original query parameters/paths in the honeypot URI
/// without breaking query string formatting.
fn url_encode(input: &str) -> String {
    let mut encoded = String::new();
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}

/// Encodes client IP addresses into a standardized 8-character hex format.
/// Supports both IPv4 and IPv6 (falling back to the last 4 bytes) to trace 
/// potential rotating proxy behaviors of malicious crawler networks.
fn format_ip_as_hex(ip_str: &str) -> String {
    if let Ok(ipv4) = ip_str.parse::<std::net::Ipv4Addr>() {
        let octets = ipv4.octets();
        format!("{:02x}{:02x}{:02x}{:02x}", octets[0], octets[1], octets[2], octets[3])
    } else if let Ok(ipv6) = ip_str.parse::<std::net::Ipv6Addr>() {
        if let Some(ipv4) = ipv6.to_ipv4() {
            let octets = ipv4.octets();
            format!("{:02x}{:02x}{:02x}{:02x}", octets[0], octets[1], octets[2], octets[3])
        } else {
            let octets = ipv6.octets();
            format!("{:02x}{:02x}{:02x}{:02x}", octets[12], octets[13], octets[14], octets[15])
        }
    } else {
        "00000000".to_string()
    }
}

// --- Askama Template structures ---

#[derive(Template)]
#[template(path = "verify.html")]
struct VerifyTemplate {
    base_timestamp: u64,
    delay_in_s: u64,
    validity_in_s: u64,
    auth_key: String,
    auth_value: String,
    crawler_link: String,
    domain: String,
    page_title: String,
    lang: String,
    colors: theme::ThemeColors,
    institution: theme::ResolvedInstitution,
    strings: theme::LocaleStrings,
}

#[derive(serde::Serialize, Clone)]
struct MockRecord {
    id: String,
    scope: String,
    value: f64,
    hash: String,
    status: String,
}

#[derive(Template)]
#[template(path = "honeypot.html")]
struct HoneypotTemplate {
    project_id: String,
    data_records: Vec<MockRecord>,
    next_page_url: String,
    session_hash: String,
}

// --- Handler Functions ---

/// GET /auth/verify
/// Renders the verification challenge form.
async fn get_verify(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let now = get_current_timestamp();
    let modifier = state.config.gatekeeper.verification_form_modifier as u64;
    if modifier == 0 {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Invalid server configuration: modifier is 0").into_response();
    }
    let modulo = now % modifier;
    
    let auth_key = format!("auth_{}", modulo);
    let expiration = now + state.config.gatekeeper.verification_form_validity_in_s;
    
    let auth_value = crypto::generate_uf(expiration, &headers, &state.config.gatekeeper.app_key);

    let ip_str = crypto::get_header_value(&headers, "x-original-ip");
    let ref_hex = format_ip_as_hex(&ip_str);

    let path_str = crypto::get_header_value(&headers, "x-original-uri");
    let path_encoded = url_encode(&path_str);

    let crawler_link = format!("/x/data?ref={}&path={}&page=1", ref_hex, path_encoded);

    let host = crypto::get_header_value(&headers, "x-original-host").to_string();
    let host = if host.is_empty() { "unknown".to_string() } else { host };

    let lang = theme::detect_language(&headers);
    let strings = theme::get_locale_strings(&lang);

    let current_theme = match state.theme.read() {
        Ok(guard) => guard.clone(),
        Err(_) => theme::Theme::default(),
    };
    let institution = current_theme.resolve_institution(&lang);

    let page_title = match &institution.name {
        Some(name) => format!("{} | {}", host, name),
        None => host.clone(),
    };

    let template = VerifyTemplate {
        base_timestamp: now,
        delay_in_s: state.config.gatekeeper.verification_form_delay_in_s,
        validity_in_s: state.config.gatekeeper.verification_form_validity_in_s,
        auth_key,
        auth_value,
        crawler_link,
        domain: host,
        page_title,
        lang,
        colors: current_theme.colors,
        institution,
        strings,
    };

    match template.render() {
        Ok(html) => (StatusCode::OK, [(header::CONTENT_TYPE, "text/html")], html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Template rendering error: {}", e)).into_response(),
    }
}

/// POST /auth/verify
/// Validates the verification challenge and sets a verified cookie.
async fn post_verify(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(payload): Json<HashMap<String, String>>,
) -> impl IntoResponse {
    let host = crypto::get_header_value(&headers, "x-original-host").to_string();
    let host = if host.is_empty() { "unknown".to_string() } else { host };

    let fail = |status: StatusCode, msg: &'static str| {
        state.increment_metric(host.clone(), |m| m.verification_failed.fetch_add(1, Ordering::Relaxed));
        (status, msg).into_response()
    };

    // Must contain exactly one key-value pair
    if payload.len() != 1 {
        return fail(StatusCode::BAD_REQUEST, "Invalid payload structure");
    }

    let (key, value) = match payload.iter().next() {
        Some(kv) => kv,
        None => return fail(StatusCode::BAD_REQUEST, "Invalid payload structure"),
    };

    // Key must start with auth_
    if !key.starts_with("auth_") {
        return fail(StatusCode::BAD_REQUEST, "Invalid verification key");
    }

    let modulo_str = &key[5..];
    let modulo: u64 = match modulo_str.parse() {
        Ok(m) => m,
        Err(_) => return fail(StatusCode::BAD_REQUEST, "Invalid verification key parameter"),
    };

    // Value must contain timestamp and hash separated by a colon
    let parts: Vec<&str> = value.splitn(2, ':').collect();
    if parts.len() != 2 {
        return fail(StatusCode::BAD_REQUEST, "Malformed verification token");
    }

    let expiration_timestamp: u64 = match parts[0].parse() {
        Ok(ts) => ts,
        Err(_) => return fail(StatusCode::BAD_REQUEST, "Malformed verification timestamp"),
    };

    let validity = state.config.gatekeeper.verification_form_validity_in_s;
    let base_timestamp = match expiration_timestamp.checked_sub(validity) {
        Some(ts) => ts,
        None => return fail(StatusCode::BAD_REQUEST, "Invalid verification timestamp"),
    };

    // 1. Verify the key modifier modulo
    let modifier = state.config.gatekeeper.verification_form_modifier as u64;
    if modifier == 0 {
        return fail(StatusCode::INTERNAL_SERVER_ERROR, "Invalid server configuration: modifier is 0");
    }
    let expected_modulo = base_timestamp % modifier;
    if expected_modulo != modulo {
        return fail(StatusCode::FORBIDDEN, "Invalid token modifier");
    }

    // 2. Verify time window constraints
    let now = get_current_timestamp();
    let delay = state.config.gatekeeper.verification_form_delay_in_s;

    // Is it expired? (now >= expiration_timestamp)
    if now >= expiration_timestamp {
        return fail(StatusCode::FORBIDDEN, "Verification token expired");
    }

    // Is it too early? (now < base_timestamp + delay)
    if now < base_timestamp + delay {
        return fail(StatusCode::FORBIDDEN, "Verification request too early");
    }

    // 3. Verify the UserFingerprint hash match
    if !crypto::verify_uf(value, &headers, &state.config.gatekeeper.app_key) {
        return fail(StatusCode::FORBIDDEN, "Verification signature mismatch");
    }

    // 4. Verification successful: Generate cookie UF
    let cookie_duration_s = state.config.gatekeeper.cookie_duration_in_days * 24 * 60 * 60;
    let cookie_expiration = now + cookie_duration_s;
    
    let cookie_uf = crypto::generate_uf(cookie_expiration, &headers, &state.config.gatekeeper.app_key);

    // Determine secure attribute dynamically based on TLS headers
    let tls_proto = crypto::get_header_value(&headers, "x-tls-protocol");
    let tls_cipher = crypto::get_header_value(&headers, "x-tls-cipher");
    let is_secure = !tls_proto.is_empty() && !tls_cipher.is_empty();

    let cookie = Cookie::build(("gk_verified_user", cookie_uf))
        .path("/")
        .http_only(true)
        .secure(is_secure)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(time::Duration::seconds(cookie_duration_s as i64))
        .build();

    state.increment_metric(host, |m| m.verification_successful.fetch_add(1, Ordering::Relaxed));
    let updated_jar = jar.add(cookie);
    (updated_jar, (StatusCode::OK, "OK")).into_response()
}

/// GET /auth/check
/// Endpoint for Nginx auth_request proxying.
async fn get_check(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    jar: CookieJar,
) -> impl IntoResponse {
    let mut response_headers = HeaderMap::new();
    response_headers.insert(header::CACHE_CONTROL, header::HeaderValue::from_static("no-store, no-cache, must-revalidate, proxy-revalidate"));
    response_headers.insert(header::PRAGMA, header::HeaderValue::from_static("no-cache"));
    response_headers.insert(header::EXPIRES, header::HeaderValue::from_static("0"));

    let is_cookie_valid = || -> bool {
        let cookie = match jar.get("gk_verified_user") {
            Some(c) => c,
            None => return false,
        };
        let cookie_val = cookie.value();
        let parts: Vec<&str> = cookie_val.splitn(2, ':').collect();
        if parts.len() != 2 {
            return false;
        }
        let expiration_timestamp: u64 = match parts[0].parse() {
            Ok(ts) => ts,
            Err(_) => return false,
        };
        let now = get_current_timestamp();
        if expiration_timestamp <= now {
            return false;
        }
        crypto::verify_uf(cookie_val, &headers, &state.config.gatekeeper.app_key)
    };

    let host = crypto::get_header_value(&headers, "x-original-host").to_string();
    let host = if host.is_empty() { "unknown".to_string() } else { host };

    let has_cookie = jar.get("gk_verified_user").is_some();

    if is_cookie_valid() {
        state.increment_metric(host, |m| m.cookie_accepted.fetch_add(1, Ordering::Relaxed));
        (response_headers, StatusCode::OK).into_response()
    } else if check_whitelist(&state, &headers) {
        state.increment_metric(host, |m| m.good_bot_passes.fetch_add(1, Ordering::Relaxed));
        (response_headers, StatusCode::OK).into_response()
    } else {
        if has_cookie {
            state.increment_metric(host, |m| m.blocks_invalid_cookie.fetch_add(1, Ordering::Relaxed));
        } else {
            state.increment_metric(host, |m| m.blocks_no_cookie.fetch_add(1, Ordering::Relaxed));
        }
        (response_headers, StatusCode::UNAUTHORIZED).into_response()
    }
}

/// GET /honeypot
/// Serves a fake research dashboard with random mock records.
async fn get_honeypot(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> impl IntoResponse {
    // Increment honeypot metric
    let host = crypto::get_header_value(&headers, "x-original-host").to_string();
    let host = if host.is_empty() { "unknown".to_string() } else { host };
    state.increment_metric(host, |m| m.honeypot_hits.fetch_add(1, Ordering::Relaxed));

    let mut rng = rand::thread_rng();
    
    // Generate mock scientific records
    let mut data_records = Vec::new();
    let scopes = vec![
        "#115",
        "#226",
        "#023",
        "#116",
        "#118",
        "#040"
    ];
    let statuses = vec!["IN REVIEW", "PENDING", "REVOKED"];

    for i in 1..=12 {
        let record_id = format!("SIID-{:04}-{:04}", rng.gen_range(1000..9999), i);
        let scope = scopes[rng.gen_range(0..scopes.len())];
        let val: f64 = rng.gen_range(0.001..99.999);
        let mut hash_bytes = [0u8; 16];
        rng.fill_bytes(&mut hash_bytes);
        let hash = hex::encode(hash_bytes);
        let status = statuses[rng.gen_range(0..statuses.len())];

        data_records.push(MockRecord {
            id: record_id,
            scope: scope.to_string(),
            value: (val * 1000.0).round() / 1000.0,
            hash,
            status: status.to_string(),
        });
    }

    let session_hash_bytes: Vec<u8> = (0..16).map(|_| rng.gen::<u8>()).collect();
    let session_hash = hex::encode(session_hash_bytes);

    // Dynamic pagination to construct the next honeypot link
    let current_page = params
        .get("page")
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(1);
    let next_page = current_page + 1;

    let mut next_params = params.clone();
    next_params.insert("page".to_string(), next_page.to_string());

    if !next_params.contains_key("ref") {
        let ip_str = crypto::get_header_value(&headers, "x-original-ip");
        let ref_hex = format_ip_as_hex(&ip_str);
        next_params.insert("ref".to_string(), ref_hex);
    }

    let mut query_parts = Vec::new();
    for (k, v) in &next_params {
        query_parts.push(format!("{}={}", url_encode(k), url_encode(v)));
    }
    // Maintain alphabetized or stable query parameters sorting to be clean
    query_parts.sort();
    let next_page_url = format!("/x/data?{}", query_parts.join("&"));

    let template = HoneypotTemplate {
        project_id: format!("PRI-AB-4026-{}", rng.gen_range(100..999)),
        data_records,
        next_page_url,
        session_hash,
    };

    match template.render() {
        Ok(html) => (StatusCode::OK, [(header::CONTENT_TYPE, "text/html")], html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Template rendering error: {}", e)).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encode() {
        assert_eq!(url_encode("abc"), "abc");
        assert_eq!(url_encode("/x/data?a=b&c=d"), "%2Fx%2Fdata%3Fa%3Db%26c%3Dd");
    }

    #[test]
    fn test_format_ip_as_hex() {
        assert_eq!(format_ip_as_hex("192.168.1.10"), "c0a8010a");
        assert_eq!(format_ip_as_hex("::1"), "00000001");
        assert_eq!(format_ip_as_hex("2001:db8::1"), "00000001");
        assert_eq!(format_ip_as_hex("::ffff:192.168.1.10"), "c0a8010a");
        assert_eq!(format_ip_as_hex("invalid"), "00000000");
    }

    #[test]
    fn test_normalize_ip() {
        assert_eq!(normalize_ip("127.0.0.1".parse().unwrap()), "127.0.0.1".parse::<std::net::IpAddr>().unwrap());
        assert_eq!(normalize_ip("::ffff:192.168.1.1".parse().unwrap()), "192.168.1.1".parse::<std::net::IpAddr>().unwrap());
        assert_eq!(normalize_ip("2001:db8::1".parse().unwrap()), "2001:db8::1".parse::<std::net::IpAddr>().unwrap());
    }

    #[test]
    fn test_ip_in_cidr() {
        let ip_v4: std::net::IpAddr = "192.168.1.50".parse().unwrap();
        let cidr_v4: std::net::IpAddr = "192.168.1.0".parse().unwrap();
        assert!(ip_in_cidr(&ip_v4, &cidr_v4, 24));
        assert!(ip_in_cidr(&ip_v4, &cidr_v4, 16));
        assert!(!ip_in_cidr(&ip_v4, &cidr_v4, 30));
        assert!(ip_in_cidr(&ip_v4, &ip_v4, 32));

        let ip_v6: std::net::IpAddr = "2001:db8:85a3::8a2e:370:7334".parse().unwrap();
        let cidr_v6: std::net::IpAddr = "2001:db8:85a3::".parse().unwrap();
        assert!(ip_in_cidr(&ip_v6, &cidr_v6, 64));
        assert!(ip_in_cidr(&ip_v6, &cidr_v6, 48));
        assert!(!ip_in_cidr(&ip_v6, &cidr_v6, 96));
        assert!(ip_in_cidr(&ip_v6, &ip_v6, 128));

        // Mixed/Mapped
        let mapped_v6: std::net::IpAddr = "::ffff:192.168.1.50".parse().unwrap();
        assert!(ip_in_cidr(&mapped_v6, &cidr_v4, 24));
    }

    #[test]
    fn test_parse_whitelist() {
        let content = "
            # Comment line
            Googlebot 66.249.64.0/19
            bingbot 157.55.39.0/24
            DuckDuckBot 104.43.54.127
            InvalidLine
        ";
        let rules = parse_whitelist_file(content);
        assert_eq!(rules.len(), 3);
        
        let google_rules = rules.get("googlebot").unwrap();
        assert_eq!(google_rules.len(), 1);
        assert_eq!(google_rules[0].0, "66.249.64.0".parse::<std::net::IpAddr>().unwrap());
        assert_eq!(google_rules[0].1, 19);

        let bing_rules = rules.get("bingbot").unwrap();
        assert_eq!(bing_rules.len(), 1);
        assert_eq!(bing_rules[0].1, 24);

        let ddg_rules = rules.get("duckduckbot").unwrap();
        assert_eq!(ddg_rules.len(), 1);
        assert_eq!(ddg_rules[0].0, "104.43.54.127".parse::<std::net::IpAddr>().unwrap());
        assert_eq!(ddg_rules[0].1, 32);
    }

    #[test]
    fn test_metrics_serialization() {
        let metrics = DomainMetrics::default();
        metrics.honeypot_hits.fetch_add(5, Ordering::Relaxed);
        metrics.blocks_no_cookie.fetch_add(10, Ordering::Relaxed);
        
        let json_str = serde_json::to_string(&metrics).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        
        assert_eq!(parsed["honeypot_hits"], 5);
        assert_eq!(parsed["blocks_no_cookie"], 10);
        assert_eq!(parsed["blocks_invalid_cookie"], 0);
        assert_eq!(parsed["cookie_accepted"], 0);
        assert_eq!(parsed["verification_successful"], 0);
        assert_eq!(parsed["verification_failed"], 0);
        assert_eq!(parsed["good_bot_passes"], 0);
    }
}

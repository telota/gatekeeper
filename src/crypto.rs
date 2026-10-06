use axum::http::HeaderMap;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Safely extracts a string representation of a header value, defaulting to an empty string.
pub fn get_header_value(headers: &HeaderMap, key: &str) -> String {
    headers.get(key)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string()
}

/// Helper to escape backslashes and colons in field values to prevent parameter injection.
fn escape_field(val: &str) -> String {
    val.replace('\\', "\\\\").replace(':', "\\:")
}

/// Helper to construct the standardized raw payload to be hashed.
fn construct_raw_payload(timestamp: u64, headers: &HeaderMap) -> String {
    let host = escape_field(&get_header_value(headers, "x-original-host"));
    let ua = escape_field(&get_header_value(headers, "user-agent"));
    let lang = escape_field(&get_header_value(headers, "x-http-lang"));
    let enc = escape_field(&get_header_value(headers, "x-http-enc"));
    let tls_proto = escape_field(&get_header_value(headers, "x-tls-protocol"));
    let tls_cipher = escape_field(&get_header_value(headers, "x-tls-cipher"));

    format!(
        "{}:{}:{}:{}:{}:{}:{}",
        timestamp, host, ua, lang, enc, tls_proto, tls_cipher
    )
}

/// Generates a User Fingerprint (UF) in the format `timestamp:hex_hash`.
pub fn generate_uf(timestamp: u64, headers: &HeaderMap, app_key: &str) -> String {
    let payload = construct_raw_payload(timestamp, headers);
    
    let mut mac = HmacSha256::new_from_slice(app_key.as_bytes())
        .expect("HMAC-SHA256 should accept keys of any size");
    mac.update(payload.as_bytes());
    let result = mac.finalize();
    let hash_hex = hex::encode(result.into_bytes());

    format!("{}:{}", timestamp, hash_hex)
}

/// Verifies that a given UF matches the expected HMAC for the current headers.
/// Uses constant-time comparison to prevent timing side-channel attacks.
pub fn verify_uf(uf: &str, headers: &HeaderMap, app_key: &str) -> bool {
    let parts: Vec<&str> = uf.splitn(2, ':').collect();
    if parts.len() != 2 {
        return false;
    }

    let timestamp_str = parts[0];
    let provided_hash_hex = parts[1];

    let timestamp: u64 = match timestamp_str.parse() {
        Ok(t) => t,
        Err(_) => return false,
    };

    let provided_hash_bytes = match hex::decode(provided_hash_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };

    let payload = construct_raw_payload(timestamp, headers);
    let mut mac = HmacSha256::new_from_slice(app_key.as_bytes())
        .expect("HMAC-SHA256 should accept keys of any size");
    mac.update(payload.as_bytes());

    mac.verify_slice(&provided_hash_bytes).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_uf_generation_and_verification() {
        let mut headers = HeaderMap::new();
        headers.insert("x-original-ip", HeaderValue::from_static("192.168.1.1"));
        headers.insert("x-original-host", HeaderValue::from_static("example.com"));
        headers.insert("user-agent", HeaderValue::from_static("Mozilla/5.0"));
        headers.insert("x-tls-protocol", HeaderValue::from_static("TLSv1.3"));
        headers.insert("x-tls-cipher", HeaderValue::from_static("TLS_AES_256_GCM_SHA384"));

        let app_key = "test_key_12345";
        let timestamp = 1718462499;

        // Generate UF
        let uf = generate_uf(timestamp, &headers, app_key);
        assert!(uf.starts_with("1718462499:"));

        // Verify with matching headers, key, and timestamp
        assert!(verify_uf(&uf, &headers, app_key));

        // Fail verification with different User-Agent
        let mut bad_headers2 = headers.clone();
        bad_headers2.insert("user-agent", HeaderValue::from_static("curl/7.68.0"));
        assert!(!verify_uf(&uf, &bad_headers2, app_key));

        // Fail verification with different key
        assert!(!verify_uf(&uf, &headers, "wrong_key"));
    }

    #[test]
    fn test_header_injection_prevention() {
        let app_key = "test_key_12345";
        let timestamp = 1718462499;

        // Case 1: normal headers
        let mut headers_normal = HeaderMap::new();
        headers_normal.insert("x-original-host", HeaderValue::from_static("example.com"));
        headers_normal.insert("user-agent", HeaderValue::from_static("Mozilla/5.0"));
        headers_normal.insert("x-http-lang", HeaderValue::from_static("de-DE"));
        headers_normal.insert("x-http-enc", HeaderValue::from_static("gzip"));

        // Case 2: injected headers attempting collision (colon injection)
        let mut headers_injected = HeaderMap::new();
        headers_injected.insert("x-original-host", HeaderValue::from_static("example.com"));
        headers_injected.insert("user-agent", HeaderValue::from_static("Mozilla/5.0:de-DE"));
        headers_injected.insert("x-http-lang", HeaderValue::from_static("gzip"));
        headers_injected.insert("x-http-enc", HeaderValue::from_static(""));

        let uf_normal = generate_uf(timestamp, &headers_normal, app_key);
        let uf_injected = generate_uf(timestamp, &headers_injected, app_key);

        // Without escaping, the payload would be:
        // "1718462499:Mozilla/5.0:de-DE:gzip:::" for both, causing collision!
        // With escaping, they must be distinct.
        assert_ne!(uf_normal, uf_injected);
    }
}


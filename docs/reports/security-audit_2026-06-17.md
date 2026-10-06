# Security Audit & Hardening Report

**Date**: 2026-06-17  
**Status**: Resolved / Implemented  
**Target Application**: Gatekeeper Daemon (Rust, Axum, Hyper)

---

## Executive Summary

A comprehensive security audit of the Gatekeeper application was conducted on June 17, 2026. The primary focus of the audit was to identify risks associated with:
1. Header/URI manipulation (parameter injection attacks).
2. Injection vulnerabilities & Denial of Service (DoS) in the `/auth/verify` endpoint.
3. Timing side-channel attacks during hash verification.
4. Resilience issues leading to daemon crashes (e.g., unhandled errors, panics, division-by-zero).

All identified issues have been successfully resolved and verified through new unit tests and compilation checks.

---

## Detailed Findings & Remediations

### 1. Header/URI Manipulation & Parameter Injection
* **Risk**: The application constructs a User Fingerprint (UF) signature by concatenating various client headers (User-Agent, Accept-Language, Accept-Encoding, TLS Protocol, TLS Cipher) using a colon (`:`) delimiter. If an attacker injects colons into header values, they can shift parameter boundaries and forge a payload that matches a target fingerprint (parameter injection/header collision).
* **Remediation**:
  - Implemented an `escape_field` helper in `src/crypto.rs` that escapes backslashes (`\` $\to$ `\\`) and colons (`:` $\to$ `\:`) within all header values before payload serialization.
  - Added the `test_header_injection_prevention` unit test to `src/crypto.rs` to guarantee that different header inputs containing colons do not result in payload collisions.

### 2. Payload Injection & DoS in `/auth/verify`
* **Risk**:
  - **Memory DoS**: The POST `/auth/verify` endpoint parsed JSON payloads without size constraints. An attacker could transmit multi-megabyte payloads to consume excessive memory, leading to Out-Of-Memory (OOM) crashes.
  - **Panic Crash**: The payload parser utilized `payload.iter().next().unwrap()`. Although preceding length checks ensured that exactly one key-value pair was present, using `.unwrap()` on client-controlled payloads represents a crash risk.
* **Remediation**:
  - Registered `axum::extract::DefaultBodyLimit::max(1024 * 16)` (16 KB limit) globally on the Axum Router in `src/main.rs`. Any request body exceeding this size is immediately rejected with `413 Payload Too Large`.
  - Refactored `payload.iter().next().unwrap()` to a safe pattern match returning `400 Bad Request` if empty, removing the panic risk.

### 3. Timing Side-Channel Attacks on Cookie Verification
* **Risk**: Hash comparison timing attacks can occur if standard string/slice equality operators (`==`) are used, which terminate comparison on the first mismatch (early out). Attackers can measure the timing differences to guess valid signatures byte-by-byte.
* **Analysis & Validation**:
  - The hash in the cookie `gk_verified_user` is validated in `get_check` via `crypto::verify_uf`.
  - Inside `verify_uf`, the signature verification is delegated to `hmac::Mac::verify_slice()`.
  - **Verdict**: The `hmac` crate's implementation of `verify_slice` compares signature slices using the `subtle` crate's `ConstantTimeEq` trait. It compares all bytes in constant time regardless of where mismatches occur. This is cryptographically secure and completely mitigates timing attacks. No code changes were needed here, but the verification has been formally documented.

### 4. Application Panics & Crash Vulnerabilities
* **Risk 1 (Division by Zero & Cryptographic Weakness)**: 
  - If `verification_form_modifier` in `config.toml` was set to `0`, the `base_timestamp % modifier` calculation triggered an uncatchable integer division-by-zero panic, crashing the daemon.
  - Furthermore, using a non-prime or too small modifier increases the risk of predictable/colliding key rotations.
* **Risk 2 (Automatic Key Generation Risks)**:
  - Automatically generating a missing `app_key` at startup hides configuration issues. If configuration changes are lost or not persisted, this could lead to silent session invalidation on restart.
* **Risk 3 (Connection Loop Crash)**: 
  - If `UnixListener::accept().await?` failed due to transient OS errors (e.g., too many open files / file descriptor exhaustion), the `?` operator propagated the error to `main`, shutting down the daemon.
* **Remediation**:
  - **Config Validation & Startup Failure**: Added a `validate(&self)` method to the `Config` struct in `src/config.rs` that validates configuration parameters at startup. It ensures:
    1. `app_key` is not empty (automatic key generation at startup has been disabled, and a missing key is now treated as a fatal startup configuration error).
    2. `verification_form_modifier` is at least 4 digits ($\ge 1000$) and a prime number (verified via a primality test helper).
    3. `socket_path` is not empty.
  - **Fatal Startup Errors**: If config validation fails, the daemon prints a clear, prominent `FATAL CONFIGURATION ERROR` message to `stderr` and exits with `1`, making it immediately visible in `systemctl start` / `systemctl status` output.
  - **Runtime Modulo Guard**: Added defensive checks `if modifier == 0` in both handlers to return `500 Internal Server Error` instead of panicking.
  - **Resilient Connection Loop**: Modified the `accept` loop in `src/main.rs` to handle errors gracefully. Instead of propagating errors, it prints them to stderr, sleeps for 100ms (to prevent high CPU busy-looping under persistent descriptor exhaustion), and continues accepting new connections.

### 5. Advanced Session Security & Infrastructure Review
* **Cookie Replay & Client IP Binding (Usability Trade-Off)**:
  - **Analysis**: IP binding was intentionally omitted from the User Fingerprint signature to prevent mobile/NAT users from being repeatedly logged out due to rotating client IP addresses. Cookie theft is instead made computationally expensive (requiring JS capabilities to solve challenges) and can be mitigated by rate-limiting requests carrying the cookie at the Nginx layer.
* **Cross-Site Request Forgery (CSRF) Mitigation**:
  - **Remediation**: Hardened the verification cookie `gk_verified_user` by setting the `SameSite=Lax` attribute during construction in `src/main.rs`. This prevents browsers from sending the authentication cookie on cross-site requests, mitigating CSRF vectors.
* **App Key Strength Verification**:
  - **Remediation**: Config validation now enforces `gatekeeper.app_key` to have a minimum length of 64 characters. This prevents operators from setting weak secrets, neutralizing offline HMAC brute-force attempts.
* **IP Spoofing Protection (Nginx Layer)**:
  - **Analysis**: Verified that in Nginx endpoints configuration (`endpoints.conf`), `X-Original-IP` is populated via `$remote_addr`. Since `$remote_addr` is set directly by Nginx from the connection's TCP header, this value cannot be spoofed by clients injecting headers.

---

## Verification & Tests

A test suite containing 7 unit tests was executed to verify all mechanisms:
1. `tests::test_format_ip_as_hex` (IP mapping stability)
2. `config::tests::test_generate_random_key` (key entropy)
3. `crypto::tests::test_header_injection_prevention` (**NEW**: parameter injection prevention)
4. `tests::test_url_encode` (percent encoding consistency)
5. `crypto::tests::test_uf_generation_and_verification` (signature lifecycle)
6. `config::tests::test_config_validation` (**NEW**: config parser guards)
7. `config::tests::test_deserialize_config_integer_and_string_keys` (config deserialization)

All tests passed successfully:
```bash
running 7 tests
test tests::test_format_ip_as_hex ... ok
test config::tests::test_generate_random_key ... ok
test crypto::tests::test_header_injection_prevention ... ok
test tests::test_url_encode ... ok
test crypto::tests::test_uf_generation_and_verification ... ok
test config::tests::test_config_validation ... ok
test config::tests::test_deserialize_config_integer_and_string_keys ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

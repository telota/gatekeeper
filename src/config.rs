use std::fs;
use std::path::Path;
use serde::{Deserialize, Deserializer};
use toml_edit::{DocumentMut, value};
use rand::RngCore;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub gatekeeper: GatekeeperConfig,
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.gatekeeper.app_key.trim().len() < 64 {
            return Err("gatekeeper.app_key must be at least 64 characters long".to_string());
        }
        let modifier = self.gatekeeper.verification_form_modifier;
        if modifier < 1000 {
            return Err("gatekeeper.verification_form_modifier must be at least 4 digits (>= 1000)".to_string());
        }
        if !is_prime(modifier) {
            return Err(format!(
                "gatekeeper.verification_form_modifier ({}) must be a prime number",
                modifier
            ));
        }
        if self.gatekeeper.socket_path.trim().is_empty() {
            return Err("gatekeeper.socket_path cannot be empty".to_string());
        }
        Ok(())
    }
}

/// Helper to determine if a number is prime.
fn is_prime(n: u32) -> bool {
    if n <= 1 {
        return false;
    }
    if n <= 3 {
        return true;
    }
    if n % 2 == 0 || n % 3 == 0 {
        return false;
    }
    let mut i = 5;
    while i * i <= n {
        if n % i == 0 || n % (i + 2) == 0 {
            return false;
        }
        i += 6;
    }
    true
}



#[derive(Debug, Deserialize, Clone)]
pub struct GatekeeperConfig {
    #[serde(deserialize_with = "deserialize_app_key")]
    pub app_key: String,
    pub verification_form_modifier: u32,
    pub verification_form_delay_in_s: u64,
    pub verification_form_validity_in_s: u64,
    pub socket_path: String,
    pub cookie_duration_in_days: u64,
    pub user_agent_list_limit: u32,
    pub user_agent_request_treshold: u32,
    pub whitelist_path: Option<String>,
    pub theme_path: Option<String>,
}

/// Custom deserializer for `app_key` to support both integers (like 123) and strings in config.toml.
fn deserialize_app_key<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let val = serde_json::Value::deserialize(deserializer)?;
    match val {
        serde_json::Value::Number(n) => Ok(n.to_string()),
        serde_json::Value::String(s) => Ok(s),
        _ => Err(serde::de::Error::custom("app_key must be a number or a string")),
    }
}

/// Loads the configuration file. Returns an error if the file does not exist or cannot be parsed.
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn std::error::Error + Send + Sync>> {
    let path_ref = path.as_ref();
    if !path_ref.exists() {
        return Err(format!("Configuration file not found: {:?}", path_ref).into());
    }
    
    let content = fs::read_to_string(path_ref)?;
    let config: Config = toml::from_str(&content)?;
    config.validate()?;
    Ok(config)
}

/// Generates a cryptographically secure 32-byte key formatted as a hex string.
pub fn generate_random_key() -> String {
    let mut key_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key_bytes);
    hex::encode(key_bytes)
}

/// Updates the `app_key` in the `config.toml` file without losing formatting or comments.
pub fn update_app_key<P: AsRef<Path>>(path: P, new_key: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path_ref = path.as_ref();
    let content = fs::read_to_string(path_ref)?;
    let mut doc = content.parse::<DocumentMut>()?;
    
    doc["gatekeeper"]["app_key"] = value(new_key);
    
    fs::write(path_ref, doc.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_config_integer_and_string_keys() {
        let toml_int = r#"
            [gatekeeper]
            app_key = 123456
            verification_form_modifier = 9973
            verification_form_delay_in_s = 2
            verification_form_validity_in_s = 60
            socket_path = "/tmp/test.sock"
            cookie_duration_in_days = 90
            user_agent_list_limit = 1000
            user_agent_request_treshold = 5
        "#;

        let config_int: Config = toml::from_str(toml_int).unwrap();
        assert_eq!(config_int.gatekeeper.app_key, "123456");

        let toml_str = r#"
            [gatekeeper]
            app_key = "secure_string_key"
            verification_form_modifier = 9973
            verification_form_delay_in_s = 2
            verification_form_validity_in_s = 60
            socket_path = "/tmp/test.sock"
            cookie_duration_in_days = 90
            user_agent_list_limit = 1000
            user_agent_request_treshold = 5
        "#;

        let config_str: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config_str.gatekeeper.app_key, "secure_string_key");
    }

    #[test]
    fn test_generate_random_key() {
        let key1 = generate_random_key();
        let key2 = generate_random_key();
        assert_eq!(key1.len(), 64); // 32 bytes hex = 64 characters
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_config_validation() {
        let invalid_toml = r#"
            [gatekeeper]
            app_key = "1234567890123456789012345678901234567890123456789012345678901234"
            verification_form_modifier = 0
            verification_form_delay_in_s = 2
            verification_form_validity_in_s = 60
            socket_path = ""
            cookie_duration_in_days = 90
            user_agent_list_limit = 1000
            user_agent_request_treshold = 5
        "#;

        let config: Config = toml::from_str(invalid_toml).unwrap();
        
        // 1. Test modifier < 1000
        let validation_res = config.validate();
        assert!(validation_res.is_err());
        assert_eq!(
            validation_res.unwrap_err(),
            "gatekeeper.verification_form_modifier must be at least 4 digits (>= 1000)"
        );

        // 2. Test short app_key
        let mut config_ok = config.clone();
        config_ok.gatekeeper.verification_form_modifier = 9973;
        config_ok.gatekeeper.app_key = "too_short_key".to_string();
        let validation_res = config_ok.validate();
        assert!(validation_res.is_err());
        assert_eq!(validation_res.unwrap_err(), "gatekeeper.app_key must be at least 64 characters long");

        // 3. Test non-prime modifier
        let mut config_ok = config.clone();
        config_ok.gatekeeper.verification_form_modifier = 1000; // 4 digits but not prime
        let validation_res = config_ok.validate();
        assert!(validation_res.is_err());
        assert_eq!(
            validation_res.unwrap_err(),
            "gatekeeper.verification_form_modifier (1000) must be a prime number"
        );

        // 4. Test empty socket_path
        let mut config_ok = config.clone();
        config_ok.gatekeeper.verification_form_modifier = 9973; // 4 digits and prime
        assert!(config_ok.validate().is_err());

        // 5. Test valid config
        config_ok.gatekeeper.socket_path = "/tmp/test.sock".to_string();
        assert!(config_ok.validate().is_ok());
    }
}

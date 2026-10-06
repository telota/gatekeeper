use std::collections::HashMap;
use std::fs;
use std::path::Path;
use axum::http::HeaderMap;
use serde::Deserialize;

use crate::crypto;

/// Evaluated theme colors passed to the template.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeColors {
    pub header_footer_bg: String,
    pub header_footer_text: String,
    pub page_bg: String,
    pub text_color: String,
    pub card_bg: String,
    pub card_border: String,
    pub btn_ready_bg: String,
    pub btn_ready_text: String,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            // Flat filled dark-gray for footer
            header_footer_bg: "#3e4955".to_string(),
            header_footer_text: "#ffffff".to_string(),
            // Clean white page background with near-black text
            page_bg: "#ffffff".to_string(),
            text_color: "#18181b".to_string(),
            // Very subtle light-gray for the card with customizable border
            card_bg: "#f4f4f5".to_string(),
            card_border: "#c1c7cd".to_string(),
            // High-contrast filled primary button
            btn_ready_bg: "#b0152d".to_string(),
            btn_ready_text: "#ffffff".to_string(),
        }
    }
}

/// Raw color structure directly deserialized from theme.toml.
#[derive(Debug, Deserialize, Default)]
struct RawColors {
    header_footer_bg: Option<String>,
    header_footer_text: Option<String>,
    page_bg: Option<String>,
    text_color: Option<String>,
    card_bg: Option<String>,
    card_border: Option<String>,
    btn_ready_bg: Option<String>,
    btn_ready_text: Option<String>,
}

/// Evaluated institution metadata per language.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InstitutionConfig {
    pub name: Option<String>,
    pub imprint_url: Option<String>,
    pub privacy_url: Option<String>,
}

/// Resolved institution fields for a specific client request.
#[derive(Clone, Debug, Default)]
pub struct ResolvedInstitution {
    pub name: Option<String>,
    pub imprint_url: Option<String>,
    pub privacy_url: Option<String>,
}

/// Fully validated and active theme instance.
#[derive(Clone, Debug, Default)]
pub struct Theme {
    pub colors: ThemeColors,
    pub institutions: HashMap<String, InstitutionConfig>,
}

impl Theme {
    /// Resolves institution details for a given requested language code.
    /// Follows strict fallback rules: Requested Lang -> English (EN) -> None (do not render).
    pub fn resolve_institution(&self, lang: &str) -> ResolvedInstitution {
        let resolve_field = |getter: fn(&InstitutionConfig) -> &Option<String>| -> Option<String> {
            // 1. Check requested language
            if let Some(inst) = self.institutions.get(lang) {
                if let Some(val) = getter(inst) {
                    if !val.trim().is_empty() {
                        return Some(val.clone());
                    }
                }
            }
            // 2. Fallback once to English if different
            if lang != "en" {
                if let Some(inst) = self.institutions.get("en") {
                    if let Some(val) = getter(inst) {
                        if !val.trim().is_empty() {
                            return Some(val.clone());
                        }
                    }
                }
            }
            // 3. Abort: do not render
            None
        };

        ResolvedInstitution {
            name: resolve_field(|i| &i.name),
            imprint_url: resolve_field(|i| &i.imprint_url),
            privacy_url: resolve_field(|i| &i.privacy_url),
        }
    }

    /// Loads theme configuration from a file or falls back cleanly to defaults.
    pub fn load_or_default(path: Option<&Path>) -> Self {
        let p = match path {
            Some(path) if path.exists() => path,
            _ => {
                println!("Notice: Theme file not found or not specified. Using built-in defaults.");
                return Self::default();
            }
        };

        let content = match fs::read_to_string(p) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Warning: Failed to read theme file {:?}: {}. Using default theme.", p, e);
                return Self::default();
            }
        };

        let raw_theme: RawTheme = match toml::from_str(&content) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Warning: Failed to parse theme file {:?}: {}. Using default theme.", p, e);
                return Self::default();
            }
        };

        let mut colors = ThemeColors::default();
        if let Some(raw_colors) = raw_theme.colors {
            if let Some(val) = raw_colors.header_footer_bg {
                match validate_color(&val) {
                    Some(c) => colors.header_footer_bg = c,
                    None => eprintln!("Warning: Invalid color for header_footer_bg '{}'. Retaining default.", val),
                }
            }
            if let Some(val) = raw_colors.header_footer_text {
                match validate_color(&val) {
                    Some(c) => colors.header_footer_text = c,
                    None => eprintln!("Warning: Invalid color for header_footer_text '{}'. Retaining default.", val),
                }
            }
            if let Some(val) = raw_colors.page_bg {
                match validate_color(&val) {
                    Some(c) => colors.page_bg = c,
                    None => eprintln!("Warning: Invalid color for page_bg '{}'. Retaining default.", val),
                }
            }
            if let Some(val) = raw_colors.text_color {
                match validate_color(&val) {
                    Some(c) => colors.text_color = c,
                    None => eprintln!("Warning: Invalid color for text_color '{}'. Retaining default.", val),
                }
            }
            if let Some(val) = raw_colors.card_bg {
                match validate_color(&val) {
                    Some(c) => colors.card_bg = c,
                    None => eprintln!("Warning: Invalid color for card_bg '{}'. Retaining default.", val),
                }
            }
            if let Some(val) = raw_colors.card_border {
                match validate_color(&val) {
                    Some(c) => colors.card_border = c,
                    None => eprintln!("Warning: Invalid color for card_border '{}'. Retaining default.", val),
                }
            }
            if let Some(val) = raw_colors.btn_ready_bg {
                match validate_color(&val) {
                    Some(c) => colors.btn_ready_bg = c,
                    None => eprintln!("Warning: Invalid color for btn_ready_bg '{}'. Retaining default.", val),
                }
            }
            if let Some(val) = raw_colors.btn_ready_text {
                match validate_color(&val) {
                    Some(c) => colors.btn_ready_text = c,
                    None => eprintln!("Warning: Invalid color for btn_ready_text '{}'. Retaining default.", val),
                }
            }
        }

        let mut institutions = HashMap::new();
        for (key, val) in raw_theme.institution {
            if let toml::Value::Table(tbl) = val {
                let inst = parse_institution_table(&key, tbl);
                institutions.insert(key.to_lowercase(), inst);
            }
        }

        // Also check if a flat [institution] block exists with direct string keys
        // (if the table contained string fields directly instead of sub-tables).
        if !institutions.contains_key("de") && !institutions.contains_key("en") {
            let mut flat_inst = InstitutionConfig::default();
            let mut has_flat = false;
            if let Ok(raw_direct) = toml::from_str::<RawDirectInstitution>(&content) {
                if let Some(direct) = raw_direct.institution {
                    if let Some(name) = direct.name {
                        if !name.trim().is_empty() {
                            flat_inst.name = Some(name.trim().to_string());
                            has_flat = true;
                        }
                    }
                    if let Some(imprint) = direct.imprint_url {
                        flat_inst.imprint_url = validate_url(&imprint);
                        if flat_inst.imprint_url.is_some() { has_flat = true; }
                    }
                    if let Some(privacy) = direct.privacy_url {
                        flat_inst.privacy_url = validate_url(&privacy);
                        if flat_inst.privacy_url.is_some() { has_flat = true; }
                    }
                }
            }
            if has_flat {
                // Place into "en" as universal fallback
                institutions.insert("en".to_string(), flat_inst);
            }
        }

        Self {
            colors,
            institutions,
        }
    }
}

/// Parses an institution key-value table into an `InstitutionConfig`.
fn parse_institution_table(lang: &str, tbl: toml::map::Map<String, toml::Value>) -> InstitutionConfig {
    let mut config = InstitutionConfig::default();

    if let Some(toml::Value::String(s)) = tbl.get("name") {
        let trimmed = s.trim();
        if !trimmed.is_empty() && trimmed.len() <= 256 {
            config.name = Some(trimmed.to_string());
        }
    }

    if let Some(toml::Value::String(s)) = tbl.get("imprint_url") {
        match validate_url(s) {
            Some(u) => config.imprint_url = Some(u),
            None => eprintln!("Warning: Invalid imprint_url for [institution.{}] '{}'", lang, s),
        }
    }

    if let Some(toml::Value::String(s)) = tbl.get("privacy_url") {
        match validate_url(s) {
            Some(u) => config.privacy_url = Some(u),
            None => eprintln!("Warning: Invalid privacy_url for [institution.{}] '{}'", lang, s),
        }
    }

    config
}

#[derive(Debug, Deserialize, Default)]
struct RawTheme {
    colors: Option<RawColors>,
    #[serde(default)]
    institution: HashMap<String, toml::Value>,
}

#[derive(Debug, Deserialize)]
struct RawDirectInstitution {
    institution: Option<RawInstitutionFields>,
}

#[derive(Debug, Deserialize)]
struct RawInstitutionFields {
    name: Option<String>,
    imprint_url: Option<String>,
    privacy_url: Option<String>,
}

/// Validates whether a given string is a safe, syntactically correct CSS color.
/// Prevents CSS injections by strictly forbidding semicolons, braces, quotes, or script tags.
pub fn validate_color(color: &str) -> Option<String> {
    let trimmed = color.trim();
    if trimmed.is_empty() || trimmed.len() > 64 {
        return None;
    }

    // Ban characters that could break CSS syntax or inject attributes
    if trimmed.contains(';')
        || trimmed.contains('{')
        || trimmed.contains('}')
        || trimmed.contains('<')
        || trimmed.contains('>')
        || trimmed.contains('"')
        || trimmed.contains('\'')
        || trimmed.contains('\\')
        || trimmed.contains('\n')
        || trimmed.contains('\r')
    {
        return None;
    }

    // 1. Hex format: #RGB, #RGBA, #RRGGBB, #RRGGBBAA
    if let Some(hex_part) = trimmed.strip_prefix('#') {
        let len = hex_part.len();
        if (len == 3 || len == 4 || len == 6 || len == 8)
            && hex_part.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Some(trimmed.to_string());
        }
        return None;
    }

    // 2. Functional CSS formats: rgb(...), rgba(...), hsl(...), hsla(...)
    let lower = trimmed.to_lowercase();
    if lower.starts_with("rgb(")
        || lower.starts_with("rgba(")
        || lower.starts_with("hsl(")
        || lower.starts_with("hsla(")
    {
        if lower.ends_with(')')
            && trimmed.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '(' | ')' | ',' | '.' | '%' | ' ' | '-'))
        {
            return Some(trimmed.to_string());
        }
        return None;
    }

    // 3. Named standard web colors
    let safe_named_colors = [
        "transparent", "currentcolor", "white", "black", "gray", "grey", "silver",
        "red", "blue", "navy", "green", "teal", "cyan", "orange", "yellow", "purple",
    ];
    if safe_named_colors.contains(&lower.as_str()) {
        return Some(lower);
    }

    None
}

/// Validates whether a given URL is a secure, valid absolute or root-relative path.
/// Strict defense against `javascript:`, data schemes, and open redirects.
pub fn validate_url(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() || trimmed.len() > 2048 {
        return None;
    }

    // Reject control characters and dangerous delimiters
    if trimmed.chars().any(|c| c.is_control() || matches!(c, '<' | '>' | '"' | '\'' | ' ' | '\t' | '\r' | '\n')) {
        return None;
    }

    // Disallow dangerous schemes
    let lower = trimmed.to_lowercase();
    if lower.starts_with("javascript:") || lower.starts_with("data:") || lower.starts_with("vbscript:") {
        return None;
    }

    // Allowed: absolute http:// or https:// URLs
    if lower.starts_with("https://") || lower.starts_with("http://") {
        return Some(trimmed.to_string());
    }

    // Allowed: root-relative paths like /assets/logo.svg, but reject protocol-relative //example.com
    if trimmed.starts_with('/') && !trimmed.starts_with("//") {
        return Some(trimmed.to_string());
    }

    None
}

/// Detects the preferred language code from request headers.
pub fn detect_language(headers: &HeaderMap) -> String {
    let raw = crypto::get_header_value(headers, "x-http-lang");
    let lower = raw.trim().to_lowercase();
    if lower.starts_with("de") {
        "de".to_string()
    } else if lower.starts_with("en") {
        "en".to_string()
    } else if lower.len() >= 2 {
        lower[..2].to_string()
    } else {
        // Fallback default
        "de".to_string()
    }
}

/// Hardcoded, tamper-proof system strings for the verification UI.
#[derive(Clone, Debug)]
pub struct LocaleStrings {
    pub page_heading: &'static str,
    pub info_text: &'static str,
    pub cookie_notice: &'static str,
    pub btn_initializing: &'static str,
    pub status_determining: &'static str,
    pub footer_description: &'static str,
    pub footer_rights: &'static str,
    pub privacy_label: &'static str,
    pub imprint_label: &'static str,
    // Dynamic JavaScript countdown strings
    pub js_btn_wait: &'static str,
    pub js_status_early_click: &'static str,
    pub js_status_wait: &'static str,
    pub js_btn_ready: &'static str,
    pub js_status_valid: &'static str,
    pub js_btn_reload: &'static str,
    pub js_status_expired: &'static str,
    pub js_alert_failed: &'static str,
    pub js_alert_error: &'static str,
}

pub fn get_locale_strings(lang: &str) -> LocaleStrings {
    if lang == "de" {
        LocaleStrings {
            page_heading: "Verifikation erforderlich",
            info_text: "Wir bieten digitale Angebote und Ressourcen für die Öffentlichkeit. Leider zwingt uns die Zunahme unregulierter Bot-Zugriffe zu erhöhten Schutzmaßnahmen. Wir bitten Sie um Verständnis für diese kurze Verifikation.",
            cookie_notice: "Hinweis: Um Missbrauch zu verhindern, wird bei erfolgreicher Verifikation ein technisch notwendiges Cookie (<code>gk_verified_user</code>) auf Ihrem Endgerät abgelegt (§ 25 Abs. 2 Nr. 2 TDDDG / Art. 6 Abs. 1 lit. f DSGVO). Es speichert ausschließlich Ihren Verifikationsstatus für nachfolgende Seitenaufrufe.",
            btn_initializing: "Initialisierung...",
            status_determining: "Status wird ermittelt...",
            footer_description: "Diese automatische Überprüfung schützt unsere Web-Infrastruktur vor missbräuchlichem Scraping.",
            footer_rights: "Alle Rechte vorbehalten.",
            privacy_label: "Datenschutz",
            imprint_label: "Impressum",
            js_btn_wait: "Bitte warten...",
            js_status_early_click: "Bitte warten Sie, bis der Countdown abgelaufen ist...",
            js_status_wait: "Verifikation verfügbar in {s}s...",
            js_btn_ready: "Verbindung verifizieren",
            js_status_valid: "Verifikation gültig für {s}s.",
            js_btn_reload: "Neu laden",
            js_status_expired: "Ihre Sitzung ist abgelaufen, bitte laden Sie die Seite neu.",
            js_alert_failed: "Verifikation fehlgeschlagen. Bitte Seite neu laden und erneut versuchen.",
            js_alert_error: "Ein Fehler ist aufgetreten. Bitte versuche es erneut.",
        }
    } else {
        LocaleStrings {
            page_heading: "Verification Required",
            info_text: "We are pleased to provide digital services and resources to the public. Unfortunately, recent developments in automated bot traffic and uncontrolled scraping force us to implement increased security measures to maintain the stability of our services. We kindly ask for your understanding regarding this brief verification.",
            cookie_notice: "Note: To prevent automated abuse, a strictly necessary cookie (<code>gk_verified_user</code>) is stored on your device upon successful verification (§ 25 (2) no. 2 TDDDG / Art. 6 (1) lit. f GDPR). It solely retains your verification state for subsequent page requests.",
            btn_initializing: "Initializing...",
            status_determining: "Determining status...",
            footer_description: "This automated check evaluates request headers to protect our web infrastructure from abusive scraping.",
            footer_rights: "All rights reserved.",
            privacy_label: "Privacy Policy",
            imprint_label: "Imprint",
            js_btn_wait: "Please Wait...",
            js_status_early_click: "Please wait until the countdown finishes...",
            js_status_wait: "Verification available in {s}s...",
            js_btn_ready: "Verify Connection",
            js_status_valid: "Verification valid for {s}s.",
            js_btn_reload: "Reload",
            js_status_expired: "Your session has expired, please reload.",
            js_alert_failed: "Verification failed. Please reload page and try again.",
            js_alert_error: "An error occurred. Please try again.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_validation() {
        assert_eq!(validate_color("#fff"), Some("#fff".to_string()));
        assert_eq!(validate_color("#3e4955"), Some("#3e4955".to_string()));
        assert_eq!(validate_color("#18181b80"), Some("#18181b80".to_string()));
        assert_eq!(validate_color("rgb(255, 0, 0)"), Some("rgb(255, 0, 0)".to_string()));
        assert_eq!(validate_color("hsl(120, 100%, 50%)"), Some("hsl(120, 100%, 50%)".to_string()));
        assert_eq!(validate_color("transparent"), Some("transparent".to_string()));

        // Reject injections and invalid colors
        assert_eq!(validate_color(""), None);
        assert_eq!(validate_color("#zzz"), None);
        assert_eq!(validate_color("#12345"), None);
        assert_eq!(validate_color("red; background: url(evil.png)"), None);
        assert_eq!(validate_color("<script>"), None);
    }

    #[test]
    fn test_url_validation() {

        // Reject dangerous schemes and malformed URLs
        assert_eq!(validate_url("javascript:alert(1)"), None);
        assert_eq!(validate_url("data:text/html,evil"), None);
        assert_eq!(validate_url("//external-domain.com"), None);
        assert_eq!(validate_url("   "), None);
        assert_eq!(validate_url("https://example.com/bad'url"), None);
    }

    #[test]
    fn test_institution_fallback_rules() {
        let mut institutions = HashMap::new();
        institutions.insert("de".to_string(), InstitutionConfig {
            name: Some("Deutsche Akademie".to_string()),
            imprint_url: Some("https://de.example/imprint".to_string()),
            privacy_url: None,
        });
        institutions.insert("en".to_string(), InstitutionConfig {
            name: Some("English Academy".to_string()),
            imprint_url: Some("https://en.example/imprint".to_string()),
            privacy_url: Some("https://en.example/privacy".to_string()),
        });

        let theme = Theme {
            colors: ThemeColors::default(),
            institutions,
        };

        // Case 1: German request gets DE properties, falling back to EN for missing privacy_url
        let de_res = theme.resolve_institution("de");
        assert_eq!(de_res.name, Some("Deutsche Akademie".to_string()));
        assert_eq!(de_res.imprint_url, Some("https://de.example/imprint".to_string()));
        assert_eq!(de_res.privacy_url, Some("https://en.example/privacy".to_string()));

        // Case 2: French request falls back once to EN
        let fr_res = theme.resolve_institution("fr");
        assert_eq!(fr_res.name, Some("English Academy".to_string()));
        assert_eq!(fr_res.imprint_url, Some("https://en.example/imprint".to_string()));
        assert_eq!(fr_res.privacy_url, Some("https://en.example/privacy".to_string()));
    }

    #[test]
    fn test_load_or_default_resilience() {
        // Missing file must cleanly return default theme without panicking
        let non_existent = Path::new("/tmp/does_not_exist_theme_xyz.toml");
        let theme = Theme::load_or_default(Some(non_existent));
        assert_eq!(theme.colors, ThemeColors::default());
        assert!(theme.institutions.is_empty());
    }

    #[test]
    fn test_theme_parsing_and_partial_overrides() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_theme_overrides.toml");

        let toml_content = r##"
            [colors]
            header_footer_bg = "#333333"
            card_border = "#e2e8f0"
            btn_ready_bg = "invalid_color_injection; drop"

            [institution.de]
            name = "Test Institut"
            imprint_url = "/imprint"
        "##;
        std::fs::write(&test_file, toml_content).unwrap();

        let theme = Theme::load_or_default(Some(&test_file));
        let _ = std::fs::remove_file(&test_file);

        // Valid colors overridden
        assert_eq!(theme.colors.header_footer_bg, "#333333");
        assert_eq!(theme.colors.card_border, "#e2e8f0");
        // Invalid color retained default
        assert_eq!(theme.colors.btn_ready_bg, ThemeColors::default().btn_ready_bg);

        // Valid relative imprint URL accepted
        let de_inst = theme.resolve_institution("de");
        assert_eq!(de_inst.name, Some("Test Institut".to_string()));
        assert_eq!(de_inst.imprint_url, Some("/imprint".to_string()));
    }

    #[test]
    fn test_corrupt_toml_fallback() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_corrupt_theme.toml");

        std::fs::write(&test_file, "this is not valid toml = = = [[[ }").unwrap();
        let theme = Theme::load_or_default(Some(&test_file));
        let _ = std::fs::remove_file(&test_file);

        assert_eq!(theme.colors, ThemeColors::default());
        assert!(theme.institutions.is_empty());
    }
}

# Configuration Reference

Gatekeeper is configured through two TOML configuration files:

1. **`config.toml`** (Required): Controls core daemon runtime parameters, cryptographic seeds, timing windows, socket bindings, and bot whitelist paths.
2. **`theme.toml`** (Optional): Customizes visual styling (colors, borders) and multi-lingual institutional branding (institution name, privacy/imprint links) for the client verification form. If omitted, built-in defaults are applied.

---

## `config.toml` Reference

The main configuration file is parsed from the daemon's working directory or passed via command-line arguments. All settings reside under the `[gatekeeper]` table.

### `[gatekeeper]` Table

#### `app_key`
* **Type:** `string`
* **Required:** Yes
* **Constraints:** Must be at least 64 characters long (typically a 32-byte hex-encoded string).
* **Default:** `""` (Empty in template; must be generated before startup)
* **Description:** Secret cryptographic key used to HMAC-SHA256 sign user fingerprints (UF) and the verification cookie (`gk_verified_user`). If this key is changed, all previously issued verification cookies immediately become invalid, forcing active visitors to reverify.

```toml   
app_key = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
```

#### `verification_form_modifier`
* **Type:** `integer (u32)`
* **Required:** Yes
* **Default:** `9967`
* **Constraints:** Must be $\ge 1000$ (at least 4 digits) and **must be a prime number**.
* **Description:** Mathematical prime modulus used to dynamically compute the dynamic token field name (`auth_<base_timestamp % modifier>`). Using a prime modulus ensures maximum dispersion across dynamic key values and prevents attackers from predicting form field names across sessions.

```toml
verification_form_modifier = 9967
```

#### `verification_form_delay_in_s`
* **Type:** `integer (u64)`
* **Required:** Yes
* **Default:** `2`
* **Unit:** Seconds
* **Description:** Enforced client-side waiting period before the verification button can be submitted. If a client submits the solution earlier than `base_timestamp + delay_in_s`, Gatekeeper rejects the request.

```toml
verification_form_delay_in_s = 2
```

#### `verification_form_validity_in_s`
* **Type:** `integer (u64)`
* **Required:** Yes
* **Default:** `60`
* **Unit:** Seconds
* **Description:** Lifespan of a generated verification challenge token. Once elapsed (`now >= expiration_timestamp`), any submission is rejected, requiring the browser to reload and request a fresh challenge.

```toml
verification_form_validity_in_s = 60
```

#### `socket_path`
* **Type:** `string`
* **Required:** Yes
* **Default:** `"/var/run/gatekeeper/gatekeeper.sock"`
* **Constraints:** Non-empty filesystem path.
* **Description:** Filesystem path where Gatekeeper binds its listening Unix Domain Socket. On startup, Gatekeeper automatically sets socket file permissions to `0666` so that unprivileged NGINX worker processes can connect without permission errors.

```toml
socket_path = "/var/run/gatekeeper/gatekeeper.sock"
```

#### `cookie_duration_in_days`
* **Type:** `integer (u64)`
* **Required:** Yes
* **Default:** `90`
* **Unit:** Days
* **Description:** Duration for which the client verification cookie (`gk_verified_user`) remains valid. After this period, the cryptographic timestamp embedded within the cookie expires, prompting NGINX to re-trigger the verification challenge on guarded routes.

```toml
cookie_duration_in_days = 90
```

#### `user_agent_list_limit`
* **Type:** `integer (u32)`
* **Required:** Yes
* **Default:** `10000`
* **Description:** Maximum number of distinct User-Agent strings tracked in memory for heuristic frequency counters and tarpit monitoring.
* **Status:** Currently **not** in use!

```toml
user_agent_list_limit = 10000
```

#### `user_agent_request_treshold`
* **Type:** `integer (u32)`
* **Required:** Yes
* **Default:** `5`
* **Description:** Request threshold for unverified User-Agent requests before rate-limiting or defensive logging counters escalate.
* **Status:** Currently **not** in use!

```toml
user_agent_request_treshold = 5
```

#### `whitelist_path`
* **Type:** `string`
* **Required:** No (Optional)
* **Default:** `"services/bot-whitelist.conf"`
* **Description:** Path to the bot IP CIDR whitelist definition file. Gatekeeper parses CIDR blocks and Bot identifiers from this file on startup. If omitted or unreadable, Gatekeeper logs an informational notice and starts with an empty whitelist without terminating.

```toml
whitelist_path = "services/bot-whitelist.conf"
```

#### `theme_path`
* **Type:** `string`
* **Required:** No (Optional)
* **Default:** `"theme.toml"` (Searched in current working directory)
* **Description:** Path to the optional `theme.toml` configuration file. If omitted or the referenced file does not exist, Gatekeeper loads its built-in default UI theme.

```toml
theme_path = "theme.toml"
```

---

## `theme.toml` Reference

The theme file defines the visual styling of the Askama verification template and provides localized institutional metadata.

> [!NOTE]
> Changes to `theme.toml` take effect immediately without dropping active socket connections by sending a `SIGHUP` signal to the daemon:
> ```bash
> pkill -HUP -x gatekeeper
> ```

### `[colors]` Table

Controls CSS custom properties rendered on the verification template (`templates/verify.html`). All values must be valid CSS color strings in hex format (`#RGB`, `#RRGGBB`, or `#RRGGBBAA`).

| Parameter | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `header_footer_bg` | `string` | `"#3e4955"` | Background color of the footer accent areas. A previuously active header has been removed. |
| `header_footer_text` | `string` | `"#ffffff"` | Text color for links and copyright notices in the header/footer. |
| `page_bg` | `string` | `"#ffffff"` | Main background color of the HTML page canvas. |
| `text_color` | `string` | `"#18181b"` | Primary font color for headings and informational copy. |
| `card_bg` | `string` | `"#f4f4f5"` | Surface background color of the central verification card. |
| `card_border` | `string` | `"#c1c7cd"` | Border color of the verification card outline. |
| `btn_ready_bg` | `string` | `"#b0152d"` | Button background color when the countdown finishes and verification is ready. |
| `btn_ready_text` | `string` | `"#ffffff"` | Button text color in the ready state. |

#### Example:
```toml
[colors]
header_footer_bg   = "#262626"
header_footer_text = "#ffffff"
page_bg            = "#ffffff"
text_color         = "#18181b"
card_bg            = "#f4f4f5"
card_border        = "#c1c7cd"
btn_ready_bg       = "#18181b"
btn_ready_text     = "#ffffff"
```

---

### `[institution.<lang>]` Tables

Defines localized institutional branding, institutional labels, and mandatory legal compliance links (Privacy Policy, Imprint) based on the visitor's `Accept-Language` header.

#### Language Resolution Strategy
1. **Direct Match:** Gatekeeper matches the primary 2-letter language code from the client's `Accept-Language` header (e.g., `de` $\to$ `[institution.de]`).
2. **English Fallback:** If the requested language table is not defined or a field is empty, Gatekeeper falls back to the English table (`[institution.en]`).
3. **Omission:** If the field is missing in both the requested language and English, the corresponding element (e.g., institution name or link) is cleanly omitted from the rendered HTML.

#### Parameters per Language Table

##### `name`
* **Type:** `string`
* **Required:** No (Optional)
* **Description:** Official name of the operating institution or project. Displayed prominently in the card header and copyright line. If unset, only the requested domain name is displayed in the heading.

##### `privacy_url`
* **Type:** `string`
* **Required:** No (Optional)
* **Description:** Absolute URL to the data protection / privacy declaration. When set, renders a clickable privacy link in the footer.

##### `imprint_url`
* **Type:** `string`
* **Required:** No (Optional)
* **Description:** Absolute URL to the legal notice / imprint (Impressum). When set, renders a clickable imprint link in the footer.

#### Example:
```toml
[institution.de]
name        = "Beispiel-Institution"
privacy_url = "https://example.example/datenschutz"
imprint_url = "https://example.example/impressum"

[institution.en]
name        = "Example-Institution"
privacy_url = "https://example.example/privacy-policy"
imprint_url = "https://example.example/imprint"
```
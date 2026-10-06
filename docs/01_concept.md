# Gatekeeper - Conceptual Architecture & Security Design

This document outlines the core architecture, threat model, and security mechanisms of the **Gatekeeper WAF** (Web Application Firewall). 


## Threat Model & Target Scraped Profiling

Modern web infrastructure hosting public data faces continuous, high-volume automated scraping. We categorize the scraping landscape into two main tiers:

* **High-Frequency, Low-Cost Scraping (Primary Threat)**
  * **Behavior:** Highly automated bots that continuously rotate IP addresses (via residential proxy networks) and spoof HTTP headers (User-Agents). 
  * **Capabilities:** Typically do *not* execute JavaScript, evaluate CSS, or maintain persistent cookie stores (state). They rely on stateless, raw HTTP requests (e.g., Python `requests`, Go `http`, or simple curl-based loops).
  * **Economic Factor:** Extremely cheap for the attacker to scale, resulting in massive request volume.

* **Full-Stack Browser Scraping (Secondary Threat)**
  * **Behavior:** Headless browsers (e.g., Playwright, Puppeteer, Selenium) or sophisticated automated agents (e.g., OpenClaw).
  * **Capabilities:** Execute full JavaScript pipelines and maintain browser state/cookies.
  * **Economic Factor:** By enforcing client-side JavaScript execution and the mandatory time delay, Gatekeeper forces the scraper out of low-cost, stateless HTTP tools and into full headless browser automation. This increases the attacker's operational footprint (memory, compute, and proxy dwell time) by orders of magnitude, making large-scale, automated extraction economically unviable.


## Core Objectives

Traditional Web Application Firewalls (like Cloudflare or Anubis) focus heavily on distinguishing *humans* from *bots*. Gatekeeper operates under a different paradigm:

* **Open Resources:** The resources protected by Gatekeeper are intended to be public. The goal is *not* to prevent access, but to **prevent resource exhaustion**.
* **Traffic Mitigation:** Massive crawl waves can saturate and overwhelm compute-intensive backend endpoints, leading to severe resource exhaustion and degraded system availability.
* **Failure of Traditional Rate Limiting:** Since scrapers rotate IPs and User-Agents rapidly, traditional IP-based rate limiting or User-Agent blacklisting fails entirely.
* **Targeted Shielding:** By applying verification only to resource-intensive paths, we shield the databases and application servers without impacting lightweight static paths.


## Architecture & Verification Flow

Gatekeeper leverages NGINX’s `auth_request` module coupled with a high-performance local daemon written in Rust, communicating over a Unix Domain Socket.

### The Cryptographic Cookie Bindings
When NGINX receives a request, it forwards the headers to Gatekeeper. If authorized, Gatekeeper evaluates the `gk_verified_user` cookie. 

To prevent replay attacks and cookie-sharing while allowing legitimate users to move across networks (e.g., changing IP from Wi-Fi to cellular):
* The cookie is **not rigidly bound to the client's IP address**.
* Instead, the cookie signature is cryptographically bound to a fingerprint composed of stable client-side parameters forwarded by NGINX:
  * **User-Agent** (`User-Agent`)
  * **TLS Protocol and Cipher Suite** (`X-TLS-Protocol`, `X-TLS-Cipher`)
  * **Language/Encoding Preferences** (`X-Http-Lang`, `X-Http-Enc`)
* By cryptographically binding the token to the negotiated TLS parameters (X-TLS-Protocol, X-TLS-Cipher), Gatekeeper neutralizes simple token exfiltration. A cookie obtained via a headless browser cannot simply be replayed in low-cost Python/Go worker pools without sophisticated TLS-stack impersonation, as mismatched cipher negotiations immediately invalidate the session at the gateway.


## Verification Challenge & Polymorphic Keys

When a client lacks a valid cookie (resulting in an HTTP `401 Unauthorized`), NGINX redirects the client to the `/verify-request` page (serving an HTTP 200 OK response). 

Rather than relying on complex third-party Captchas (which degrade UX and introduce external dependencies), Gatekeeper enforces a **resource-delay challenge**:

* **Wait-Time Constraint:** The client must wait a configurable duration (default: 2 seconds) before the verification request becomes valid.
* **Polymorphic Cryptographic Keys:** The verification form generates dynamic, time-limited cryptographic request parameters. The client-side script submits these via a POST request using a dynamically changing payload structure (polymorphic keys).
* **Bot Defeated by Design:**
  * Simple, stateless scrapers cannot execute the JavaScript required to generate the signature and post the request.
  * Even if a headless browser script is used, it is forced to wait the full 2 seconds per instance before obtaining a valid cookie. This directly neutralizes the speed benefit of parallel, rapid scraper pools.


## Cookie Security & Rate Limiting

* **Strict Cookie Lifespan:** Once generated, the cookie is sign-verified against a server-side `app_key` and has a strict expiry date (which is signed within the cookie and not fakeable).
* **Cookie-Based Rate Limiting:** Within NGINX, we can apply rate limits specifically to the `gk_verified_user` cookie value (instead of the IP). This ensures that even if a bot successfully acquires a cookie, its request volume is strictly throttled, and it cannot simply spawn new IPs to bypass the limit.


## Core Philosophy: Shifting the Economic Asymmetry

Attackers exploit an asymmetric advantage: sending a request is virtually free for them, whereas serving a database-heavy page is expensive for our server. 

Gatekeeper reverses this equation:
1. **Force Verification:** To access intensive resources, the client must perform work (time delay, JS evaluation, state retention).
2. **Remove Anonymity:** Legitimate access requires a minimal verification token. IP- or Setup-Shift is pointless.
3. **Impose Cost:** By forcing a 2-second wait time and JS evaluation, the cost of scraping our resources at high volume scales quadratically for the attacker, rendering massive, aggressive crawls economically unfeasible.


## GDPR Compliance & Privacy by Design

To ensure full compliance with the General Data Protection Regulation (GDPR / DSGVO), Gatekeeper implements privacy by design:
* **Intentional Over-Compliance & Active Agency:** While infrastructure defense and DDoS mitigation qualify as a strictly necessary measure (exempting the token from consent under Art. 5(3) ePrivacy / national telemedia acts), Gatekeeper deliberately opts for an affirmative consent model.
* **Active User Consent:** The tracking/verification cookie (`gk_verified_user`) is **never set automatically** on the initial visit. It is only stored after the user explicitly triggers it by clicking the "Verify Connection" / "Verbindung verifizieren" button, representing affirmative user action.
* **Strict Minimization:** No personal data is stored inside the cookie or logged persistently by default. The cookie simply contains a cryptographically signed validation token and its timestamp.


## Footer-Hidden Honeypot

To bind automated scraper pipelines that attempt to bypass the verification flow:
* **Deceptive Link Baiting:** A hidden link ("For Crawlers") pointing to the honeypot path is embedded in the footer of the verification page. This link is styled to be completely non-interactive for human users (using CSS properties that suppress interaction including `aria-hidden`), but remains fully readable in the DOM for HTML parsers and web scrapers. Explicit robot-tags prevent search engines (and *polite* crawlers)from consuming the link or the target page.
* **Honeypot Trap:** Any request hitting this specific honeypot route is intercepted by NGINX and routed to Gatekeeper. Since no human user could naturally discover or click this link, any access is concidered to be a bot/scraper.
* **Slowing down the Scraper:** Using NGINX's `limit_rate` extends the response time by at least 1 second, which effectively consolidates the scraper's computing time.
* **Threat Intelligence & Mitigation:** Even if the Gatekeeper itself does not maintain a blacklist, an administrator could easily set up an NGINX's `access_log` for the honeypot and thus create a list of scrapers that could be used as the basis for an automated blacklist or for analysis. We have not implemented this by default for data protection reasons.


## Dynamic Bot Whitelisting & Zero-Downtime Reload

To allow search engine crawlers (e.g., Googlebot, Bingbot, DuckDuckBot) to index public academic resources without being blocked by the verification flow, Gatekeeper implements a dynamic bot whitelisting mechanism:
* **Feed-based CIDR Validation:** Simply checking the HTTP `User-Agent` is insecure, as User-Agents can be easily spoofed. Gatekeeper validates both the `User-Agent` (against known crawler substrings) and matching IP ranges.
* **Efficient In-Memory Matching:** At startup, CIDR ranges are parsed from `bot-whitelist.conf` and stored in an in-memory `HashMap` grouped by User-Agent keys. When a stateless request arrives, Gatekeeper performs a fast User-Agent substring check. If a match occurs, it enforces the IP validation against the allowed CIDR ranges for that bot (early-abort on mismatch).
* **Automated Feed Updates:** A seperate cron script (`helper.py`) fetches the latest official IP ranges from crawler feeds (Google, Bing, DuckDuckGo) and generates the configuration file. To avoid writing to disk and reloading the daemon unnecessarily, updates are only written if the feed content has changed.
* **SIGHUP Integration:** To guarantee zero downtime, Gatekeeper listens for the Unix `SIGHUP` signal. Upon receiving the signal, it reloads and parses `bot-whitelist.conf` in a separate worker thread, replacing the in-memory whitelist atomically without interrupting active request validation.


## Lightweight Internal Dashboard & Historical Metrics

To monitor system activity, block counts, and traffic patterns without introducing heavy database dependencies or complex analytics stacks:
* **Lock-Free In-Memory Counters:** The Gatekeeper daemon tracks metrics on a per-domain basis using atomic integers (`AtomicU64`). It exposes these raw counters at `/auth/metrics` as structured JSON.
* **Stateless Persistence & Aggregator:** The hourly cron helper script fetches the in-memory metrics via the local Unix Domain Socket. It computes hourly differences, accounts for daemon restarts (by checking the daemon's start time), and appends them to a rolling 7-day history file (`metrics_history.json`).
* **Intranet-Restricted Dashboard:** A lightweight HTML5 dashboard (`dashboard.html`) renders the daily overview and weekly activity charts (via responsive inline SVGs). To ensure high security, access is strictly limited to localhost and private intranet IP ranges (RFC 1918) directly at the Nginx level.


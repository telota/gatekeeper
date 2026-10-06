# TELOTA Gatekeeper: A cryptographic, GDPR-compliant bot mitigation service for NGINX, written in Rust.

 *Gatekeeper* is a lightweight, resource-efficient daemon written in Rust designed to curb unregulated bot traffic. It is engineered to operate seamlessly alongside NGINX in Linux environments, running locally on the reverse proxy server. Communication between NGINX and  *Gatekeeper* is handled via a high-performance Unix Domain Socket.

> [!IMPORTANT]  
> *Gatekeeper* is **not** a full security suite. Its sole objective is to neutralize low-cost scraping, shielding downstream application servers and databases from resource exhaustion.
 
## Specifications

| Dimension | Implementation Details |
| :--- | :--- |
| **Licence** | Apache 2.0 |
| **Tech Stack** | Rust (Axum, Tokio), Python for helper scripts |
| **Integration** | systemd service unit |
| **Binary Size** | ~4.1 MB (optimized release build) |
| **Memory Footprint** | ~5 MB RAM |
| **NGINX Connection** | `auth_request` over Unix Domain Socket |
| **Mechanism** | Client connection fingerprint stored in a cryptographic HMAC cookie |
| **Data Management** | Stateless in-memory operation (no database required) |
| **Metrics** | Lock-free atomic counters (`AtomicU64`), asynchronous retrieval via helper script, intranet dashboard |
| **Privacy Compliance** | 100% GDPR / TDDDG compliant (privacy-by-design) |
| **SEO Compatibility** | Automated IP/CIDR whitelisting for Google, Bing, and DuckDuckGo |
| **On-Failure Mode** | Fail-open (ensures zero downtime for backend services) |

## Docker Showcase
Our showcase Docker container provides a hands-on look at practical usage without requiring a local installation of Rust or the Gatekeeper.
```bash
docker compose up --build
```

> [!NOTE]
> The Docker build takes a moment because it first needs to pull the Rust image and compile the binary. After that, the container will be ready within seconds, even after a restart.
>
> This setup is strictly for demonstration purposes and is not intended for production use!

Once running, the showcase will be available at `http://localhost:8080`:

* `/` demonstrates the *Gatekeeper* integration.  
  After successful verification, all subsequent requests are forwarded directly to the actual backend.
* `/live-metrics` exposes the internal counters of the *Gatekeeper*.  
  In production, you would typically use an external service (or the included [helper script](services/helper.py) and [dashboard](services/dashboard.html)) to collect and analyze these metrics.
* `/x/data` serves as an integrated honeypot (tarpit).  
  Any aggressive bot that scrapes the verification form while ignoring robots directives will end up in this maze of random strings—burning its compute budget without putting stress on the real endpoints.

By default, the showcase is configured to use port 8080. You can easily override this default using a `.env` file:
```bash
echo 'SHOWCASE_PORT=8081' > .env
```
 
## Architecture & Request Flow

```text
Client ──> NGINX ──(auth_request / UDS)──> Gatekeeper Daemon (Rust/Axum)
             │                                    │
             ├── (200) ──> Backend Server         ├── Good Bot (verified CIDR range)?
             └── (401) ──> /verify-request        └── Cookie & Fingerprint valid?
```
See the [NGINX template](docs/implementation/nginx/gatekeeper.conf.template) to understand how *Gatekeeper* is integrated.

Unlike traditional Web Application Firewalls (such as Cloudflare or Anubis-Techaro),  *Gatekeeper* does not attempt to categorize every visitor as "human or bot" through intrusive CAPTCHAs or behavioral heuristics. Instead,  *Gatekeeper* operates on **cryptographic connection fingerprinting**. 

From stable request headers and the TLS handshake,  *Gatekeeper* constructs an individual, signed HMAC cookie (`gk_verified_user`). Traditional, low-cost stateless scrapers cannot acquire this cookie because they do not execute JavaScript or manage persistent cookie jars. Conversely, cookies harvested by expensive headless browser instances cannot be transferred to cheaper scraping networks because the network and TLS fingerprints between the two setups differ. If this mitigation alone is insufficient, server administrators can enforce rate limiting directly on the  *Gatekeeper* cookie within NGINX, completely eliminating the advantage of rotating residential proxy IPs.

* **SEO-Friendly:** Search engine crawlers that publish verified IP ranges (Googlebot, Bingbot, DuckDuckBot) are explicitly whitelisted and bypass verification. Nonetheless, we recommend applying  *Gatekeeper* specifically to locations typically excluded in `robots.txt` or to resources that cannot be effectively cached (e.g., faceted searches and complex database aggregations) to maintain maximum accessibility.
* **100% GDPR-Compliant:** The verification cookie is fully anonymized, serves a legitimate interest, and is set only after affirmative action by the user (§ 25 (2) No. 2 TDDDG / Art. 6 (1) (f) GDPR).  *Gatekeeper* does not retain persistent request logs; it records only aggregate, anonymous access counters per domain.

## Is Gatekeeper Right for My Use Case?

The answer depends on your available resources and the level of protection you need. Gatekeeper is primarily designed for smaller infrastructure where aggressive bot traffic is not just an annoyance, but a critical threat to server uptime—yet established solutions like Cloudflare or Anubis are not an option (whether due to cost, privacy compliance, or infrastructure constraints). This scenario is typical across the GLAM sector (Galleries, Libraries, Archives, Museums) and the digital humanities.

To deploy Gatekeeper, root/admin access to the reverse proxy server (or its container) is required. The protected resources should inherently be public open-access data: Gatekeeper does not secure confidential assets; its sole purpose is to preserve infrastructure performance by shedding unmanaged, low-cost bot requests. 

Gatekeeper is **not** suitable for websites that explicitly want or need to be indexed by unknown or long-tail crawlers, nor is it a replacement for API key management. Even though major search engines (Google, Bing, DuckDuckGo) are whitelisted, presence in smaller search indices and accessibility for third-party aggregators may be reduced. As a general rule, Gatekeeper should only protect endpoints where application- or server-level caching cannot be reasonably implemented.
 
## Background & Philosophy

Rising automated bot traffic has been a known challenge for academic infrastructure. Historically, we managed traffic spikes at the Berlin-Brandenburg Academy of Sciences and Humanities (BBAW) with standard tools. Since late 2025, however, bot volumes have escalated dramatically. Academic infrastructure is fundamentally not provisioned for the extreme request volumes we now observe. We evaluated existing solutions like Anubis, but integrating them into our existing NGINX topology would have introduced significant configuration complexity and a new potential bottleneck by routing all production traffic through container networks.

Our resources are **Open Access**: we want humans, search engine crawlers, and AI agents to access our public scholarly data freely. What we cannot support are aggressive gray-market data harvesters that ignore `robots.txt` and disregard rate limits. These operators trigger millions of automated requests using spoofed User-Agents across residential proxy networks—rendering traditional IP- or User-Agent-based blocking ineffective.

 *Gatekeeper* was built out of pure pragmatism to achieve maximum protective impact with minimal operational overhead. We have run  *Gatekeeper* in continuous production since mid-2026. Across the first 100 days on two production domains, it successfully deflected over **150 million bot requests**.  *Gatekeeper* is not designed to be an impenetrable fortress, nor does it need to be. The scraper economy relies on marginal costs near zero. Scrapers will not rewrite custom scraping infrastructure for academic repositories, and headless browser pipelines remain constrained by their steep compute costs.
 
## Documentation

* **[Concept](./docs/01_concept.md)**  
  Architectural overview, threat model, and cryptographic design
* **[Performace](./docs/reports/performance-audit_2026-06-22.md)**  
  An example run of the [benchmark](./benchmark) analysed
* **[Real-World Metrics](./docs/04_metrics.md)**  
  Production metrics and performance analysis from our servers
---
* **[Integration](./docs/02_integration.md)**  
  Production deployment guide, systemd setup, and NGINX configuration
* **[Configuration](./docs/03_configuration.md)**  
  Configuration und customization guide
* **[Local Development](./docs/05_local-development.md)**  
  Quickstart guide for setting up a local development environment
---
* **[Planned Features](./docs/06_planned-features.md)**  
  Roadmap and upcoming improvements
* **[Contributing](./CONTRIBUTING.md)**  
  Guidelines for community contributions and bug reporting

## Citation

```bibtex
@software{Koster_telota_gatekeeper_2026,
  author = {Köster, Jan},
  month = oct,
  title = {{TELOTA Gatekeeper: A cryptographic, GDPR-compliant bot mitigation service for NGINX, written in Rust.}},
  url = {https://github.com/telota/gatekeeper},
  year = {2026}
}
```
 
## AI Disclosure

 *Gatekeeper* was developed with AI assistance (Gemini in Google Antigravity). Human prompts, design decisions, and architectural iterations are documented in [`docs/intents`](./docs/intents).  
Portions of the documentation were initially drafted in German and subsequently translated, or developed collaboratively with AI tooling.

## DISCLAIMER OF WARRANTIES AND LIMITATION OF LIABILITY

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.


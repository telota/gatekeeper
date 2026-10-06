# AGENTS.md

## 1. Projektübersicht
- **Zweck:** unternehmensweite, föderierte Sicherheits-Infrastruktur (Enterprise Distributed WAF)
- **Tech-Stack:** Rust (Axum, Tokio).
- **Architektur:** 
  - Der Gatekeeper läuft auf einem unix-Socket
  - NGINX bittet den Gatekeeper via `auth_request` um die Prüfung eines Requests (Gatekeeper prüft, ob ein verfied-cookie vorliegt - wenn nicht, ob der UserAgent und die IP zu einem "good" Bot gehört, etc.)
  - Der Gatekeeper stellt ein HTML-Formular und einen Endpoint, damit ein menschlicher User bei einem gescheiterten `auth_request` einen Verfikations-Cookie bekommen kann
  - Er stellt eine HTML-Seite, die als Honeypot für Scraper fungiert
  - Der Gatekeeper loggt im Ram die Zugriffe gegen die Ressourcen (atomicU64-Counter)
  - Es gibt einen separaten Python-Job (`services/helper.py`), der als Cron regelmäßig die Live-Metriken ausliest und die Whitelist für die "good" Bots schreibt.
  - Es gibt ein von Rust getrenntes HTML-Dashboard, das die vom Helper ermittelten Metriken visualisiert
  - Die Lebensdauer des Verifikations-Cookies wird über die `config.toml` geregelt, der Default sind 90 Tage

## 2. Anforderungen
- Wir brauchen maximale Performance, damit der Dienst auch bei hohem Request-Aufkommen nicht zum Flaschenhals wird.
- Der Dienst ist stateless.
- Der Gatekeeper liest die `config.toml` (und optional die `theme.toml`) für seine Konfiguration beim Start/Reload.

## 3. Constraints (Grenzen)
*Diese Bereiche sind für dich absolut tabu, es sei denn, ich fordere dich explizit dazu auf:*
- Verändere **niemals** Dateien in den Ordnern `.git`.
- `new-intent.sh` darf weder verändert noch ausgeführt werden.
- Files in `docs/intents` dürfen nur nach ausdrücklicher Aufforderung verändert werden.


# Internal Dashboard & Metrics Logging

2026-06-17

**Status: finished**

## Tasks

Wir wollen ein ressourcenschonendes, minimalistisches Dashboard bereitstellen, das die Trends der letzten 7 Tage visualisiert (aufgeteilt nach Domains/Hosts). Der Daemon selbst bleibt schreibgeschützt und zustandslos, während ein stündliches Cronjob die Persistenz übernimmt.

Zur Vereinfachung des Deployments werden alle Hilfsdateien in einem neuen Ordner `services/` am Repository-Root verwaltet.

### 1. In-Memory Metriken & Uptime im Gatekeeper-Daemon
* Aufzeichnung des Start-Zeitpunkts des Daemons beim Hochfahren (`start_time` als Unix-Timestamp).
* Implementierung von `DomainMetrics` im `AppState` unter Verwendung von `AtomicU64` zur lock-freien Erfassung von 7 Kern-Metriken pro Host (erfasst über den Header `X-Original-Host`):
  * `honeypot_hits`: Zugriffe auf den honeypot-Endpunkt.
  * `blocks_no_cookie`: 401-Antworten bei fehlendem Cookie.
  * `blocks_invalid_cookie`: 401-Antworten bei vorhandenem, aber abgelaufenem/ungültigem Cookie.
  * `cookie_accepted`: Erfolgreiche Cookie-Validierung (Zugriff erlaubt).
  * `verification_successful`: Gelöstes Krypto-Challenge-Rätsel.
  * `verification_failed`: Ungültiger Verifizierungs-Request.
  * `good_bot_passes`: Erlaubte Zugriffe verifizierter Suchmaschinen-Bots.

### 2. Endpunkte im Daemon
* `/auth/metrics` (JSON): Liefert die aktuellen, rohen Counter-Werte aller aktiven Domains sowie das Feld `start_time` direkt aus dem Arbeitsspeicher.

### 3. Nginx-Konfiguration
* Erstellung eines neuen Nginx-Snippets unter `docs/implementation/nginx/snippets/gatekeeper/dashboard.conf` (nur Locations, kein Server-Block):
  * `/auth/dashboard` $\rightarrow$ liefert die statische Datei `dashboard.html` aus (im Dev-Betrieb aus `services/dashboard.html`, in Prod aus `/etc/gatekeeper/dashboard.html`).
  * `/auth/dashboard/history` $\rightarrow$ liefert die JSON-Datei `metrics_history.json` aus (im Dev-Betrieb aus `services/metrics_history.json`, in Prod aus `/etc/gatekeeper/metrics_history.json`).
* **Intranet-Beschränkung (IP-basiert):**
  * Zugriff auf beide Pfade wird via Nginx `allow`/`deny` Direktiven auf lokale Loops (`127.0.0.1`, `::1`) und private Subnetze (RFC 1918: `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`) beschränkt. Öffentlicher Zugriff wird blockiert (`deny all;`).

### 4. Cron-Helper Skript (`services/helper.py`)
* Ein einziges stündliches Python-Skript vereint die Aufgaben:
  1. Abruf der Bot-IPs und Update der `services/bot-whitelist.conf` (mit anschließendem `SIGHUP` an den Daemon).
  2. Abruf der JSON-Werte von `/auth/metrics` und Fortschreiben der `services/metrics_history.json`.
* **Persistenz, Reset-Handling & Uptime:**
  * Berechnung der Differenz zwischen dem aktuellen Wert und dem Stand des letzten stündlichen Abrufs.
  * Erkennt ein Absinken des Werts (Reset bei Daemon-Neustart) und wertet den neuen Stand als Differenz seit Neustart.
  * Schreibt den aktuellen `start_time` des Daemons in die History-Datei, um Neustarts für das Dashboard sichtbar zu machen.
  * Addiert die stündlichen Differenzen auf den heutigen Tag in `metrics_history.json` auf.
  * Hält eine rollierende Historie der letzten 7 Tage (Entfernen des ältesten Tages-Eintrags, falls ein neuer Tag anbricht).
  * Das Schreiben der Datei erfolgt atomar (Schreiben in `.tmp` mit `chmod 644` und anschließendes `mv` nach `metrics_history.json`).

### 5. Client-side Dashboard (`services/dashboard.html`)
* Das im Browser des Admin-Users geladene JavaScript holt sich die Historie aus `/auth/dashboard/history` und rendert die Entwicklung der letzten 7 Tage sowie den aktuellen Daemon-Uptime-Status interaktiv im Browser.

## Additions
* **Trend-Extrapolation (Tagesverlauf):** Berechnet die Tendenz des aktuellen Tages im Vergleich zum Durchschnitt der letzten 6 vollständigen Tage. Um Verzerrungen in den frühen Morgenstunden zu vermeiden, wird der aktuelle Tageswert anhand der verbleibenden Tageszeit hochgerechnet (Verstreichen des Tages in Prozent, abgedeckt mit einem minimalen Schwellenwert von 2% zur Vermeidung von Divisionen durch Null).
* **Glassmorphic Terminal Layout (Dark UI):** Vollständiges CSS im edlen, dunklen Terminal-Design (Dunkelgrau/Silber) mit transparentem Backdrop-Filter (Blur) und feinen Übergangseffekten. 
* **Englische Lokalisierung:** Vollständige Übersetzung aller UI-Strings (Valid Cookies, Blocked (No Cookie), Blocked (Invalid), Verified, Failed, Good Bots, Honeypot) und Entwicklerkommentare im Dashboard-Code zur Wahrung der Repository-Konsistenz.

## Debriefing
* **Domain Details Overlay-Modal:** Zur Minimierung der initialen Datenmenge und besseren Übersichtlichkeit wird standardmäßig nur die Gesamt-Tagesübersicht gelistet. Die vollständige Wochenübersicht (inkl. SVG-Balkendiagramm) wird bei Klick auf eine Domain in ein Modal-Fenster ausgelagert.
* **7-Spalten-Spiegelung im Grid:** Das Modal-Grid spiegelt exakt dieselben 7 Metrik-Spalten der Tabelle wider. Über ein CSS-Media-Query ab 992px Bildschirmbreite wird erzwungen, dass alle Kacheln sauber nebeneinander in einer Reihe aufgereiht sind.
* **Vollständig lokale Ressourcen:** Entfernung aller externen Webfonts/CSS-Ressourcen (Google Fonts), um ein Laden im geschlossenen Intranet sicherzustellen.
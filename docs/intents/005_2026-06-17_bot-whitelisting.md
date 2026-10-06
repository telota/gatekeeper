# Bot Whitelisting

2026-06-17

**Status: finished** 

## Tasks

Aktuell lässt der Gatekeeper niemanden ohne Cookie durch. Für SEO etc. ist das ein Albtraum. Wir brauchen einen Whitelisting-Mechanismus.
* Wir erstellen eine neue bot-whitelist.conf. Sie enthält UserAgents (bzw. Strings darin wie "Google") und die IP-Ranges dahinter, um Spoofing zu verhindern
* Diese Liste wird beim Startup in den RAM geladen.
* Ein Request ohne Cookie wir auf den UserAgent und die zugelassene IP geprüft.
* Diese Prüfung muss sehr schnell sein, da sie bei jedem Request stattfindet.

Fragen & Antworten: 
* Welche "guten" Bots sollten wir durchlassen? Google ist klar, vermutlich noch Bing und/oder Duckduckgo. Gibt es noch andere?
  * *Antwort:* Googlebot, bingbot und DuckDuckBot wurden als Kern-Suchmaschinen implementiert.
* Woher kommt diese Liste? Meines Wissens stellen Google und co so etwas zur Verfügung. Wie oft rotieren die IPs? Reicht für den Start eine statische Liste, die wir händisch reloaden können oder müssen wir einen komplexeren Mechanismus realisieren?
  * *Antwort:* Die Listen werden dynamisch über ein Bash-Skript (`update-whitelist.sh`) aus den offiziellen JSON-Feeds der Betreiber generiert. Über `SIGHUP` kann die Liste im laufenden Betrieb geladen werden.

## Additions
* **Automatisches Update-Skript (`update-whitelist.sh`):** Holt sich die aktuellsten IP-Ranges per HTTP-GET von den offiziellen Provider-Feeds, generiert die `bot-whitelist.conf` atomar und triggert den Hot-Reload im Daemon über `SIGHUP`.
* **SIGHUP Integration:** Ein asynchroner Signal-Handler lauscht auf `SIGHUP`, um die Whitelist im Betrieb komplett unterbrechungsfrei neu einzulesen.

## Debriefing
* **Gruppierte In-Memory HashMap:** Statt einer flachen Liste werden die CIDR-Ranges nach User-Agent-Substrings (`"googlebot"`, `"bingbot"`, `"duckduckbot"`) in einer `HashMap` gruppiert. Dadurch sind pro Request maximal 3 Substring-Suchen nötig.
* **Early-Abort Spoofing-Schutz:** Matcht ein User-Agent ein Pattern (z. B. `"googlebot"`), wird die Suche sofort auf diesen Key fixiert. Ist die IP-Adresse nicht in der dazugehörigen Liste, bricht die Prüfung direkt ab (Block) – es erfolgt keine unnötige Überprüfung anderer Bot-Typen.
* **Natives CIDR-Matching:** Vollständig in Rust ohne Third-Party-Crates (`ipnet` o. Ä.) über bitweise Operatoren gelöst.


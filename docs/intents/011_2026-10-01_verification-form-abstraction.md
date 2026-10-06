# Verification Form Abstraction & Theming

2026-10-01

**Status: finished**

## Zielsetzung
Das Verification-Formular soll für Dritte (insbesondere im GLAM-Sektor: Bibliotheken, Archive, Museen) ohne Re-Kompilierung des Binaries anpassbar werden. 
Um Sicherheitsrisiken und Support-Aufwände zu vermeiden, bleibt das rohe HTML/JavaScript-Gerüst unantastbar. Betreiber können stattdessen über eine optionale Konfigurationsdatei (`theme.toml`) eine ausgewählte Menge an Farben, Links und Institutions-Metadaten steuern. 

Zentrales Prinzip: **Resilienz & Zero-Panic.** Weder die Konfigurationsdatei noch einzelne Felder darin sind verpflichtend. Ungültige oder fehlende Werte werden geloggt und fallen geräuschlos auf fest verdrahtete Defaults zurück.

---

## Tasks

### 1. Design vereinfachen und neue Defaults
- Vollständigen Wechsel auf ein sauberes Light-Theme
- Entfernen aller Border-Effekte und -Definitionen (flat filled design)
- Header und Footer bekommen denselben neuen BG-Farbwert (Akzent), Default ist ein dunkles Grau, Schriftfarbe bleibt für Header und Footer per default auf Weiß, ist aber anpassbar
- wir definieren den Hintergrund der Seite als uni-Weiß und die Schriftfarbe als ein Fast-Schwarz, Die Verification-Card bekommt ein sehr, sehr helles Grau.
- Wir definieren das aktuelle Verification-Logo (das Schild mit dem Häkchen) als Default. Es wird (sofern gegeben) durch das Institutslogo aus der Config überschrieben.
- `<title>` - wir definieren den Domainnamen und den jeweiligen Namen der Institution (siehe Config) als Titel. Das sieht weniger unschön aus, wenn jemand einen Link irgendwo postet (aktuell erscheint in der Vorschau immer "Verifikation erforderlich")

### 2. Neues Modul `src/theme.rs`
- **Structs definieren:**
  - `Theme`: Enthält die vollständig evaluierten Werte zur Übergabe an das Template (Farben, Links, Logos). Implementiert `Default`
  - `RawTheme`: Deserialisierungs-Struct für `theme.toml`, in dem alle Felder `Option<String>` sind.
- **Validatoren & Sanitizer implementieren:**
  - `validate_color(value: &str) -> Option<String>`: Prüft Hex-Codes (`#RGB`, `#RRGGBB`, `#RRGGBBAA`) oder standardisierte CSS-Farbwerte. Verwirft ungültige Werte und potenzielle CSS-Injections.
  - `validate_url(value: &str) -> Option<String>`: Validiert absolute URLs (`https://`) oder sichere relative Pfade (`/assets/...`). Sperrt unsichere Schemes (z. B. `javascript:`).
- **Laderoutinen mit Fallback:**
  - `Theme::load_or_default(path: Option<&Path>) -> Theme`: 
    - Existiert die Datei nicht: `Info`-Log, Rückgabe von `Theme::default()`.
    - Ist das TOML syntaktisch defekt: `Warn`-Log mit Fehlerbeschreibung, Rückgabe von `Theme::default()`.
    - Ungültige Einzelfelder: `Warn`-Log pro Feld, betroffenes Feld fällt auf den Wert aus `Theme::default()` zurück (da viele Felder standardmäßig leer sind, resultiert es in einem Nicht-Render).

---

### 3. Konfigurationsdatei `theme.toml` & Template-Entwurf
- **Struktur von `theme.toml` festlegen:**
  - `[colors]`: Reduktion auf die neue Kern-Palette.
  - `[institution]`: Name der Einrichtung, optionaler Logo-Pfad/URL, Link zum Impressum, Link zum Datenschutz. Wenn diese Felder leer sind, werden sie im Frontend gar nicht gerendert
- **Beispieldatei anlegen:**
  - `theme.toml.example` mit Dokumentation der verfügbaren Optionen und Defaults.
- **Pfad-Einbindung:**
  - In `config.toml` optionales Feld `theme_path` ergänzen (Default-Fallback auf `./theme.toml`).

---

### 4. Zukunftsfähige Mehrsprachigkeit (i18n-Vorbereitung)
- **Trennung System- vs. Institutions-Texte:**
  - System-Texte (Countdown, Wartehinweise, Status-Labels) verbleiben im Binary und werden über typisierte Sprachkataloge aufgelöst (z. B. `de`, `en`).
  - Institutions-Texte (Name der Einrichtung, Links für Impressum, Datenschutz und Logo) können in `theme.toml` optional pro Sprache hinterlegt werden. Wird eine Sprache gefordert, die nicht definiert ist, fällt das Form auf Englisch zurück (oder wenn gar keine Values definiert sind, wird der entsprechende Eintrag gar nicht gerendert)
- **Template-Entschlackung:**
  - Schrittweise Vorbereitung des Askama-Templates: Entfernen binärer `{% if is_german %}`-Verschachtelungen zugunsten übergebener Sprach-Strukturen und konsistenter Platzhalter.

---

### 5. Integration & Template-Anpassung
- **`src/main.rs` entschlacken:**
  - Theme-Laden beim Start und Halten der Referenz im `AppState`.
  - Hot-Reloading: Bei `SIGHUP` neben der Bot-Whitelist auch das Theme neu einlesen.
- **`templates/verify.html` bereinigen:**
  - CSS-Variablen im `:root`-Block dynamisch aus dem `Theme`-Objekt befüllen.
  - Optionale Slots für Logo und Footer-Links (Impressum/Datenschutz) einbinden (nur rendern, wenn Link konfiguriert).

---

### 6. Qualitätssicherung & Tests
- **Unit-Tests in `src/theme.rs`:**
  - Test auf gültige/ungültige Farbdefinitionen (Hex, Injection-Strings).
  - Test auf gültige/ungültige Links und relative Pfade.
  - Test auf partielle `theme.toml` (nur ein Key gesetzt, Rest greift auf `Default` zurück).
  - Test auf defektes TOML (darf nicht panicken, muss vollständigen `Default` liefern).
- **Regressions-Test:**
  - Vollständiger Lauf von `cargo test` und manueller Verifikations-Flow über Unix-Socket.

---

## Additions

Folgende architektonische und gestalterische Anpassungen wurden im Zuge der Umsetzung zusätzlich eingeführt:
- **Card-Border & Customizing**: Einführung eines konfigurierbaren Rahmens (`card_border = "#c1c7cd"` in `[colors]`), um die helle Card bei Bedarf klarer vom Seitenhintergrund abzugrenzen.
- **Header-Eliminierung**: Der obere `<header>`-Balken wurde komplett entfernt, um den visuellen Fokus vollständig auf die zentrale Verifikations-Card zu legen.
- **Verzicht auf Logos / Icons**: Streichung des Shield-SVGs und der Bild-Slots (`logo_url`). Stattdessen setzt sich der Kopf der Card aus dem Institutionsnamen (sofern konfiguriert) und dem prominenten Domainnamen zusammen.
- **Dynamische Typografie-Skalierung (`fitText`)**:
  - Kombination aus CSS Container Queries (`container-type: inline-size` auf `.card`, `clamp` mit `cqi`-Einheiten) für unmittelbares Layouting ohne Cumulative Layout Shift.
  - Clientseitiges JavaScript (`fitText`), das die Schriftgröße von Institutions- und Domainname bei Fenstergrößenänderungen stufenlos skaliert, sodass beide Angaben garantiert auf einer einzelnen Zeile verbleiben.
  - `overflow: hidden; text-overflow: ellipsis;` als Fallback gegen Zeilenumbrüche oder Clipping.
- **Strikte Fallback-Kaskade (EN-Only)**: Konsequente Umsetzung der Sprachauflösung: Fehlt eine angefragte Sprache (z. B. `fr`), wird einmalig `en` versucht; ist auch Englisch nicht hinterlegt, wird der Eintrag vollständig unterdrückt (kein fehlerhafter Fallback auf Deutsch).
- **Bereinigung von Dead Code**: Sämtliche verbliebenen `logo_url`-Felder und -Routinen wurden restlos aus `src/theme.rs`, `templates/verify.html` und der Beispielkonfiguration entfernt.

---

## Debriefing

### Architektur & Komponenten
1. **Modul `src/theme.rs`**:
   - `ThemeColors` kapselt alle CSS-Farbvariablen mit robusten Defaults.
   - `InstitutionConfig` und `ResolvedInstitution` verwalten lokalisierte Metadaten (`name`, `imprint_url`, `privacy_url`).
   - `validate_color` und `validate_url` filtern XSS- und Injection-Versuche (z. B. `javascript:`, Semikolons, Braces).
   - `LocaleStrings` trennt System-Texte (Countdown, Status, Button-Labels für `de` und `en`) vollständig von veränderbaren Institutions-Texten.
2. **Konfiguration**:
   - `theme.toml` steuert Branding und Farben; wird über `.gitignore` vom VCS ausgeschlossen.
   - `theme.toml.example` dokumentiert alle verfügbaren Optionen und die Fallback-Mechanismen.
   - `config.toml` erlaubt über `theme_path` die optionale Pfadfestlegung.
3. **Integration in `src/main.rs`**:
   - Theme wird beim Bootstrapping thread-safe in `AppState` (`RwLock<Theme>`) hinterlegt.
   - Hot-Reloading: Bei Empfang von `SIGHUP` wird `theme.toml` atomar neu geladen, ohne dass der Daemon neu gestartet werden muss.

### Tests & Verifikation
- **Unit-Tests**: 17 automatisierte Tests in `cargo test` validieren Farb- und URL-Sanitization, resiliente Fallbacks bei korruptem oder fehlendem TOML sowie die Mehrsprachigkeitskaskade.
- **Kompilierung**: Clean Release-Build ohne Warnungen via `cargo build --release`.

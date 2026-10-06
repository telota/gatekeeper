# Verification Frontend

2026-06-15

**Status: finished** 

## Tasks

Anmerkung: wenn ich dir im folgenden Strings vorgebe, dann ist das für das Verification-Form immer zweisprachig gedacht (übersetze es einfach entsprechend). Achten auf eine gewisse Höflichkeit.

Wir müssen ein paar Frontend-Optimierungen vornehmen:
- Bitte ergänzen überall, sofern noch nicht geschehen, Robots-Meta-tags für No-Follow, No-Index und No-Archive
- Verification-Form:
  - Der Button darf während der initialen Wartezeit nicht gesperrt sein (er soll zwar durch graue Farbe so aussehen, aber clickbar sein, um headless Browser zum Click zu verleiten). Ein Click triggert ein Zurücksetzen der Wartezeit um 2 Sekunden. Der Text des Counters färbt sich rot.
  - Wenn das Zeitfenster ausläuft, verzichte auf ein confirm und zeige im Button stattdessen "Reload" an. Der Text darunter informiert "Your Session expired, please reload"
  - Ändere "Gatekeeper" im Header oben links zu "Berlin-Brandenburgische Akademie der Wissenschaften" bzw. "Berlin-Brandenburg Academy of Sciences and Humanities", analog dazu die Bezeichnung nach dem Copyright-Hinweis im Footer.
  - Link für das Impressum `https://www.bbaw.de/impressum` / `https://www.bbaw.de/en/imprint`
  - Link für das Privacy-Statement `https://www.bbaw.de/datenschutz` / `https://www.bbaw.de/en/privacy-policy`
  - Link zu APIs entfernen
  - Ergänze einen Hinweis, dass bei einem Click auf Verify aus technischen Gründen ein Cookie gesetzt werden muss (begründe es mit Verweis auf die DSGVO als technische Notwendigkeit)
  - Überarbeite noch einmal die Texte, die auf die Scraper-Problematik hinweisen. Wir stellen unsere Angebote kostenlos bereit und versuchen, die Unanehmlichkeiten so gering wie möglich zu halten. Leider zwingen uns die aktuellen Entwicklungen zu erhöhten Schutzmaßnahmen. wir bitten um Verständnis.

## Debriefing

### Umsetzung der Anforderungen:

1. **Robots-Tags:** In `<head>` von `verify.html` und `honeypot.html` wurde `<meta name="robots" content="noindex, nofollow, noarchive">` eingefügt.
2. **Rebranding & Links:**
   - Header und Footer verweisen nun auf die **Berlin-Brandenburgische Akademie der Wissenschaften** (BBAW) bzw. **Berlin-Brandenburg Academy of Sciences and Humanities** (zweisprachig per Askama).
   - Die Impressum- und Datenschutzlinks wurden auf die offiziellen BBAW-URLs umgestellt und targeten im neuen Tab (`target="_blank"`). Der API-Link wurde entfernt.
3. **Scraper-Hinweistext:** Höflich überarbeitet, um Verständnis gebeten und die Serverstabilität als Schutzgrund angeführt.
4. **DSGVO Cookie-Hinweis:** Textlich direkt über dem Button platziert und auf die technische Notwendigkeit (Art. 6 Abs. 1 lit. f DSGVO) verwiesen.
5. **Bot-Wartezeit-Falle:**
   - Der Button hat kein `disabled` und ist klickbar (`cursor: pointer` auch für `.waiting`).
   - Klicks in der Wartezeit setzen `elapsedSeconds` auf `0` zurück, was den 2-Sekunden-Countdown neu auslöst.
   - Der Counter-Text färbt sich rot (`.text-error`) und warnt: *"Bitte warten Sie, bis der Countdown abgelaufen ist..."* (DE) / *"Please wait until the countdown finishes..."* (EN). Nach Ablauf der Sekunde normalisiert sich die Anzeige wieder und der reguläre Countdown läuft weiter.
6. **Session-Ablauf:**
   - Verzicht auf `confirm()`-Popups.
   - Button färbt sich rot (`.expired`) und ändert den Text auf *"Neu laden"* / *"Reload"*.
   - Der Text darunter informiert über den Ablauf. Ein Klick lädt die Seite direkt neu (`location.reload()`).
# Security Audit

2026-06-17

**Status: finished** 

## Tasks

Wir wollen die Anwendungen auf mögliche Schwachstellen bzw. Sicherheitslücken prüfen und diese schließen.
* Wie verhält sich die Anwendung, wenn ein Angreifer manipulierte Header oder URIs einschmuggelt?
* Können über den Post-Paylod des Verification-Forms (/auth/verify) Injection Attacken performed werden?
* Wie erfolgt der String-Vergleich beim Check des Hash? Müssen wir hier eventuell absolute Timing-Attacken bedenken? Lösung: `subtle` und Checks ohne frühzeitigen Abbruch?
* Wird irgendwie `unwrap()` oder `expect()` genutzt, was einen Panik-Crash verursachen könnte? Falls ja, müssen wir das abfangen
* Prüfe auf weitere mögliche Schwachstellen/Angriffsszenarien

Schreibe einen ausführlichen Audit in `docs/reports/security-audit_2026-06-17.md`

## Debriefing

Bericht erstellt und die Schwachstellen behoben.

# Verification Form Redesign

2026-06-18

**Status: finished** 

## Tasks

Bitte entferne `Logo` und `System Status` aus dem Header der `verify.html`. Ersetze es durch ein zentriertes Element, das die aktuelle Domain anzeigt (NGINX-Header `X-Original-Host`). Skaliere da Element so, dass es auf kleinen Bildschirmen ohne Zeilenumbruch oder Overflow darstellbar ist.

Wir müssen den NGINX-Header `X-Original-Host` in unseren Cookie-Fingerprint und Form-Fingerprint übernehmen, um bei mehreren Domains eine Übertragbarkeit des Cookies zu verhindern. Sei hier bitte gründlich, um Divergenzen zwischen Erstellung/Verifikation und Check zu vermeiden.

Bitte prüfe, wo im Code wir `html_response_delay_in_ms` aus der config.toml verwenden oder ein Delay/Sleep von 400ms hardcodiert ist - und entferne diese Anweisungen anschließend. Ich beziehe mich hier ausschließlich auf Anweisungen, die den Rust-Dienst zur Verzögerung der Response zwingen. Wir wollen sie entfernen, um bei großem Traffic die Zahl der offenen Verbindungen zu verringern.

Bitte ergänze in `docs/implementation/system/gatekeeper.service` eine Logik, die bei einem Panic oder Absturz des Dienstes einen Restart nach 100ms erzwingt. Bau ein, dass der Dienst nicht unendlich oft neu gestartet wird, um einen infinite restart loop zu verhindern.

## Debriefing

- **JS-Race-Condition / Time-Overlap**: Es wurde festgestellt, dass der Button zur Verifikation im Challenge-Formular (`templates/verify.html`) nach genau 1,0 Sekunden Echtzeit fälschlicherweise freigegeben wurde, da die UI-Aktualisierung `tick()` beim Laden der Seite den internen Zähler `elapsedSeconds` sofort auf `1` erhöhte. Bei Klick durch den Benutzer trat ein Zeit-Überlauf auf dem Server auf, was zu einer Ablehnung mit `403 Forbidden` (Verification request too early) führte. Dies wurde durch die `Date.now()`-Umstellung behoben. Dies garantiert, dass die Restlaufzeit des Delays und der Gültigkeit exakt mit den Serverzeitstempeln übereinstimmt.


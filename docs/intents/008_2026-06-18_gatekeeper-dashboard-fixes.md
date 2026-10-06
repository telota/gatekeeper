# Gatekeeper Dashboard Fixes

2026-06-18

**Status: finished** 

## Tasks

Die Tendenzen im Dashboard werden falsch berechnet.

```
Domain / Host	Valid Cookies	Blocked (No Cookie)	Blocked (Invalid)	Verified	Failed	Good Bots	Honeypot
domain_1	0(±0)	260(+692)	0(±0)	0(±0)	0(±0)	81(+188)	0(±0)
domain_2	31(-38)	86757(+235387)	0(±0)	1(-5)	0(-1)	19(+52)	87412(+229627)
```
Schau mal bitte in die `services/metrics_history.json` und dann in `services/dashboard.html`.
Die Idee war, dort einen Prozentwert anzuzeigen und nicht die Werte aus `last_scapred_counters`. Dieser Prozentwert sollte so berechnet werden: Werte des heutigen Tages geteilt durch die aktuelle Uhrzeit in Stunden * 24 h (z.b. 24/7), um den noch unvollständigen Tageswert zu extrapolieren. Anschließend nimmst du den Durchschnitt der letzten verfügbaren Tage, teilst den extrapolierten Tageswert durch diesen Durchschnitt, multiplizierst mit 100 und ziehst 100 ab, um die prozentuale Tendenz zu ermitteln. Führe ein hard Cap bei +/-999% ein.

Bitte lege für die Tendenzen eigene Spalten an und lass das Spaltenlabel jeweils beide Spalten spannen. Wenn du dann die Textausrichtung in der Spalte für die Tendenzen auf links änderst, bekommen wir eine schöne saubere Optik.
Die absoluten Werte sollen mit Punkten als Tausender-Trenner angezeigt werden

Du musst dir keine anderen Dateien ansehen. Dieses Problem betrifft nur die Dashboard.html

## Debriefing

Wie gefordert umgesetzt

#!/bin/bash

# Ziel-Ordner definieren und sicherstellen, dass er existiert
TARGET_DIR="./docs/intents"
mkdir -p "$TARGET_DIR"

# Titel abfragen (Eingabeaufforderung nach stderr umleiten für sauberen stdout-Output)
echo -n "Titel eingeben: " >&2
read -r raw_title

# Titel in lower kebab-case umwandeln
# xargs trimmt führende/nachfolgende Leerzeichen und kollabiert mehrere Blanks zu einem einzigen
clean_title=$(echo "$raw_title" | tr '[:upper:]' '[:lower:]' | xargs | sed 's/ /-/g')

# Zeitstempel generieren (Y-m-d)
timestamp=$(date +%Y-%m-%d)

# Höchsten existierenden 3-stelligen Index ermitteln
highest=$(find "$TARGET_DIR" -maxdepth 1 -name "[0-9][0-9][0-9]_*" -exec basename {} \; 2>/dev/null | sort | tail -n 1 | cut -d'_' -f1)

if [ -z "$highest" ]; then
    next_index=1
else
    # 10# verhindert, dass Bash führende Nullen als Oktalzahl interpretiert
    next_index=$((10#$highest + 1))
fi

# Index mit führenden Nullen auf 3 Stellen formatieren
index_padded=$(printf "%03d" $next_index)

# Dateiname zusammensetzen
filename="${index_padded}_${timestamp}_${clean_title}.md"

# Markdown-Datei mit Template befüllen
cat << EOF > "$TARGET_DIR/$filename"
# ${raw_title}

${timestamp}

**Status: pending** 

## Tasks

[Description]
*
EOF

# Ausschließlicher Output auf stdout
echo "${filename} was created!"
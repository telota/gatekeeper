#!/bin/bash
set -euo pipefail

# Determine script directory to resolve default file locations
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXAMPLE_FILE="$SCRIPT_DIR/config.toml.example"
TARGET_FILE="${1:-$SCRIPT_DIR/config.toml}"

# Ensure template exists
if [ ! -f "$EXAMPLE_FILE" ]; then
    echo "Error: Template file '$EXAMPLE_FILE' not found." >&2
    exit 1
fi

# Guard against accidental overwrites
if [ -e "$TARGET_FILE" ]; then
    echo "Error: Configuration file '$TARGET_FILE' already exists. Aborting." >&2
    exit 1
fi

# Generate 32-byte (64 hex characters) cryptographically secure key
if command -v openssl >/dev/null 2>&1; then
    NEW_KEY=$(openssl rand -hex 32)
else
    NEW_KEY=$(head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n')
fi

# Validate key length
if [ "${#NEW_KEY}" -ne 64 ]; then
    echo "Error: Failed to generate a valid 64-character hex key (length: ${#NEW_KEY})." >&2
    exit 1
fi

# Copy template to destination
cp "$EXAMPLE_FILE" "$TARGET_FILE"

# Inject generated key into target configuration
sed -i "s|^[[:space:]]*app_key[[:space:]]*=.*|app_key = \"$NEW_KEY\"|" "$TARGET_FILE"

# Verify successful insertion
if ! grep -q "^[[:space:]]*app_key[[:space:]]*=[[:space:]]*\"$NEW_KEY\"" "$TARGET_FILE"; then
    echo "Error: Failed to insert app_key into '$TARGET_FILE'." >&2
    rm -f "$TARGET_FILE"
    exit 1
fi

echo "Successfully created configuration file: $TARGET_FILE"
echo "Generated and inserted secure 64-character app_key."

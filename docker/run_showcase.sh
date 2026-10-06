#!/bin/bash
set -e

echo ""
echo "=== Starting Gatekeeper Showcase Setup ==="
echo ""

# Create runtime directory for the Unix Domain Socket
mkdir -p /var/run/gatekeeper

# Start Gatekeeper daemon in background
echo "Initialising Gatekeeper configuration..."

if [ ! -f /app/config.toml ] && ! /app/set-config.sh; then
    echo "Gatekeeper configuration failed." >&2
    exit 1
fi
echo "Configuration initialised."

cd /app
echo "Starting Gatekeeper daemon..."
./gatekeeper &
GATEKEEPER_PID=$!

# Ensure Gatekeeper is terminated when container stops
cleanup() {
    echo "Stopping Gatekeeper..."
    kill -TERM "$GATEKEEPER_PID" 2>/dev/null || true
}
trap cleanup EXIT SIGTERM SIGINT

# Wait for Unix socket to become available
echo "Waiting for Gatekeeper Unix socket..."
for i in {1..30}; do
    if [ -S /var/run/gatekeeper/gatekeeper.sock ]; then
        echo "Gatekeeper socket is ready."
        break
    fi
    sleep 0.2
done

if [ ! -S /var/run/gatekeeper/gatekeeper.sock ]; then
    echo "ERROR: Gatekeeper socket was not created in time."
    exit 1
fi

sleep 1

echo ""

echo "Validating NGINX configuration..."
if ! nginx -t -c /app/nginx.conf; then
    echo "ERROR: NGINX configuration test failed. Aborting." >&2
    exit 1
fi

echo "NGINX configuration is valid."
echo ""

sleep 0.5

echo "=========================================="
echo "Showcase ready. Visit http://localhost:8080"
echo ""

# Run NGINX in foreground to keep container PID 1 alive
exec nginx -g "daemon off;" -c /app/nginx.conf





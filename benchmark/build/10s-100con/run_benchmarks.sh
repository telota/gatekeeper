#!/bin/bash
set -e

echo "=== Starting Gatekeeper 10k-requests Benchmark Setup ==="

# Create runtime directory for the Unix Socket
mkdir -p /var/run/gatekeeper

# Start Gatekeeper in /app, redirecting log to a temporary file inside the container
cd /app
echo "Starting Gatekeeper..."
./gatekeeper > /tmp/gatekeeper.log 2>&1 &
GATEKEEPER_PID=$!

# Start Nginx using the scenario configuration
echo "Starting Nginx..."
nginx -c /app/nginx.conf &
NGINX_PID=$!

# Wait for the Unix socket to appear
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
    cat /tmp/gatekeeper.log
    exit 1
fi

sleep 1

# === Test 1: Baseline (Guardless) ===
echo "--- Running Test 1: Baseline (Guardless) ---"
python3 /app/monitor.py --output /tmp/test1_stats.json &
MONITOR_PID=$!
sleep 0.5

# Unlimited requests over 10s: Concurrency 100
hey -z 10s -c 100 http://localhost/unguard > /tmp/test1_hey.log

kill -INT $MONITOR_PID
wait $MONITOR_PID || true
sleep 2

# === Test 2: Gatekeeper Fast Path (No Cookie) ===
echo "--- Running Test 2: Gatekeeper Fast Path (No Cookie) ---"
python3 /app/monitor.py --output /tmp/test2_stats.json &
MONITOR_PID=$!
sleep 0.5

hey -z 10s -c 100 http://localhost/guard > /tmp/test2_hey.log

kill -INT $MONITOR_PID
wait $MONITOR_PID || true
sleep 2

# === Test 3: Gatekeeper Slow Path (Valid Cookie / Signature Check) ===
echo "--- Running Test 3: Gatekeeper Slow Path (Valid Cookie / Signature Check) ---"
FUTURE_TS=$(($(date +%s) + 2592000))
COOKIE_VAL="${FUTURE_TS}:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"

python3 /app/monitor.py --output /tmp/test3_stats.json &
MONITOR_PID=$!
sleep 0.5

hey -z 10s -c 100 -H "Cookie: gk_verified_user=${COOKIE_VAL}" http://localhost/guard > /tmp/test3_hey.log

kill -INT $MONITOR_PID
wait $MONITOR_PID || true
sleep 2

# === Dump Metrics ===
# echo "Dumping Gatekeeper metrics..."
# curl -s --unix-socket /var/run/gatekeeper/gatekeeper.sock http://localhost/auth/metrics > /results/gatekeeper_metrics.json || true

# === Tear down processes ===
echo "Stopping services..."
kill $NGINX_PID 2>/dev/null || true
kill $GATEKEEPER_PID 2>/dev/null || true

# === Generate Report ===
echo "Generating Markdown Report..."
python3 /app/report.py /results

echo "=== Benchmark Complete ==="

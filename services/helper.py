#!/usr/bin/env python3
import os
import sys
import json
import socket
import http.client
import urllib.request
import subprocess
from datetime import datetime

class UnixHTTPConnection(http.client.HTTPConnection):
    """Custom HTTP connection to fetch data over a Unix Domain Socket."""
    def __init__(self, unix_socket_path):
        super().__init__('localhost')
        self.unix_socket_path = unix_socket_path

    def connect(self):
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.connect(self.unix_socket_path)

def find_config(script_dir):
    """Finds config.toml in the parent directory relative to the script."""
    p = os.path.abspath(os.path.join(script_dir, '../config.toml'))
    if os.path.exists(p):
        return p
    return None

def parse_config(config_path):
    """Simple parser to extract keys from config.toml."""
    config = {}
    if not config_path:
        return config
    try:
        with open(config_path, 'r') as f:
            for line in f:
                line = line.strip()
                if '=' in line and not line.startswith('#'):
                    k, v = line.split('=', 1)
                    config[k.strip()] = v.strip().strip('"').strip("'")
    except Exception as e:
        print(f"Warning: Failed to read config file {config_path}: {e}", file=sys.stderr)
    return config

def read_feeds(conf_path):
    """Reads crawler feeds from crawler-feeds.conf."""
    feeds = {}
    if os.path.exists(conf_path):
        try:
            with open(conf_path, 'r') as f:
                for line in f:
                    line = line.strip()
                    if line and not line.startswith('#'):
                        parts = line.split(None, 1)
                        if len(parts) == 2:
                            feeds[parts[0]] = parts[1]
        except Exception as e:
            print(f"Warning: Failed to read feeds config {conf_path}: {e}", file=sys.stderr)
    return feeds

def update_whitelist(feeds, whitelist_path):
    """Fetches IP ranges for each bot and updates the whitelist file only if changed."""
    print("Updating crawler IP whitelist...", file=sys.stderr)
    if not feeds:
        print("No crawler feeds configured. Whitelist will be cleared.", file=sys.stderr)
    
    new_content_lines = ["# Gatekeeper bot IP whitelist. Generated automatically.\n\n"]
    failed_any = False
    
    for bot, url in feeds.items():
        print(f"Fetching {bot} from {url}...", file=sys.stderr)
        try:
            req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0 (Gatekeeper Whitelist Updater)'})
            with urllib.request.urlopen(req, timeout=10) as response:
                data = json.loads(response.read().decode('utf-8'))
                prefixes = data.get("prefixes", [])
                
                new_content_lines.append(f"# --- {bot} ranges ---\n")
                count = 0
                for prefix_entry in prefixes:
                    cidr = prefix_entry.get("ipv4Prefix") or prefix_entry.get("ipv6Prefix")
                    if cidr:
                        new_content_lines.append(f"{bot} {cidr}\n")
                        count += 1
                new_content_lines.append("\n")
                print(f"Successfully processed {count} ranges for {bot}.", file=sys.stderr)
        except Exception as e:
            print(f"Error fetching {bot} from {url}: {e}", file=sys.stderr)
            failed_any = True
            break
            
    if failed_any:
        print("Error: Whitelist update aborted because one or more feeds failed to download.", file=sys.stderr)
        print("The existing whitelist file remains untouched.", file=sys.stderr)
        return

    new_content = "".join(new_content_lines)
    
    # Compare with existing file content
    existing_content = ""
    if os.path.exists(whitelist_path):
        try:
            with open(whitelist_path, 'r') as f:
                existing_content = f.read()
        except Exception as e:
            print(f"Warning: Failed to read existing whitelist file: {e}", file=sys.stderr)
            
    if new_content == existing_content:
        print("No changes detected in crawler IP whitelist. Skipping write and SIGHUP signal.", file=sys.stderr)
        return
        
    temp_path = whitelist_path + ".tmp"
    try:
        with open(temp_path, 'w') as f:
            f.write(new_content)
        
        os.chmod(temp_path, 0o644)
        os.replace(temp_path, whitelist_path)
        print(f"Whitelist written to {whitelist_path}", file=sys.stderr)
        
        # Trigger reload via SIGHUP
        try:
            res = subprocess.run(["pkill", "-HUP", "-x", "gatekeeper"], capture_output=True)
            if res.returncode == 0:
                print("Sent SIGHUP reload signal to gatekeeper daemon.", file=sys.stderr)
        except Exception as e:
            print(f"Could not send SIGHUP: {e}", file=sys.stderr)
            
    except Exception as e:
        print(f"Failed to write whitelist: {e}", file=sys.stderr)
        sys.exit(1)

def fetch_metrics(socket_path):
    """Fetches raw metrics JSON from the daemon Unix socket."""
    if not os.path.exists(socket_path):
        print(f"Error: Socket file does not exist at {socket_path}", file=sys.stderr)
        return None
    
    conn = UnixHTTPConnection(socket_path)
    try:
        conn.request('GET', '/auth/metrics')
        resp = conn.getresponse()
        if resp.status == 200:
            return json.loads(resp.read().decode('utf-8'))
        else:
            print(f"Metrics endpoint returned status code: {resp.status}", file=sys.stderr)
            return None
    except Exception as e:
        print(f"Error connecting to daemon socket: {e}", file=sys.stderr)
        return None
    finally:
        conn.close()

def update_metrics_history(metrics, history_path):
    """Updates the metrics history file, calculating diffs and handling resets."""
    print("Updating metrics history...", file=sys.stderr)
    
    # Load existing history
    history_data = {
        "last_start_time": 0,
        "last_scraped_counters": {},
        "history": []
    }
    
    if os.path.exists(history_path):
        try:
            with open(history_path, 'r') as f:
                history_data = json.load(f)
        except Exception as e:
            print(f"Warning: Failed to parse history file {history_path}: {e}. Reinitializing.", file=sys.stderr)

    curr_start_time = metrics.get("start_time", 0)
    curr_domains = metrics.get("domains", {})

    last_start_time = history_data.get("last_start_time", 0)
    last_scraped = history_data.get("last_scraped_counters", {})
    history_list = history_data.get("history", [])

    # We use local time for daily metrics aggregation
    today_str = datetime.now().strftime("%Y-%m-%d")

    # Find today's entry
    today_entry = None
    for entry in history_list:
        if entry.get("date") == today_str:
            today_entry = entry
            break
            
    if today_entry is None:
        today_entry = {"date": today_str, "domains": {}}
        history_list.append(today_entry)

    # Detect daemon restart
    restarted = (curr_start_time != last_start_time)
    if restarted:
        print("Daemon restart detected. Calculating metrics from zero offset.", file=sys.stderr)

    # Calculate differences and accumulate
    for domain, curr_metrics in curr_domains.items():
        if domain not in today_entry["domains"]:
            today_entry["domains"][domain] = {k: 0 for k in curr_metrics.keys()}
            
        last_metrics = last_scraped.get(domain, {})
        domain_entry = today_entry["domains"][domain]

        for metric_name, curr_val in curr_metrics.items():
            if restarted:
                diff = curr_val
            else:
                last_val = last_metrics.get(metric_name, 0)
                if curr_val >= last_val:
                    diff = curr_val - last_val
                else:
                    diff = curr_val # Treat as reset fallback
                    
            domain_entry[metric_name] = domain_entry.get(metric_name, 0) + diff

    # Keep a rolling window of the last 7 days
    history_list.sort(key=lambda x: x.get("date"))
    if len(history_list) > 7:
        history_list = history_list[-7:]

    new_history = {
        "last_start_time": curr_start_time,
        "last_scraped_counters": curr_domains,
        "history": history_list
    }

    # Write atomically
    temp_path = history_path + ".tmp"
    try:
        with open(temp_path, 'w') as f:
            json.dump(new_history, f, indent=2)
        os.chmod(temp_path, 0o644)
        os.replace(temp_path, history_path)
        print(f"Metrics history successfully updated in {history_path}", file=sys.stderr)
    except Exception as e:
        print(f"Error saving history file: {e}", file=sys.stderr)

def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    base_dir = script_dir

    # Output paths
    whitelist_path = os.path.join(base_dir, 'bot-whitelist.conf')
    history_path = os.path.join(base_dir, 'metrics_history.json')
    feeds_conf_path = os.path.join(base_dir, 'crawler-feeds.conf')

    # 1. Update Whitelist
    feeds = read_feeds(feeds_conf_path)
    update_whitelist(feeds, whitelist_path)

    # 2. Update Metrics History
    config_path = find_config(script_dir)
    config = parse_config(config_path)
    socket_path = config.get("socket_path", "/var/run/gatekeeper/gatekeeper.sock")
    
    # Check if socket path is relative and make it absolute relative to config directory
    if socket_path and not socket_path.startswith('/'):
        if config_path:
            config_dir = os.path.dirname(os.path.abspath(config_path))
            socket_path = os.path.abspath(os.path.join(config_dir, socket_path))

    metrics = fetch_metrics(socket_path)
    if metrics:
        update_metrics_history(metrics, history_path)
    else:
        print("Could not fetch metrics from daemon. Skipping history update.", file=sys.stderr)

if __name__ == '__main__':
    main()

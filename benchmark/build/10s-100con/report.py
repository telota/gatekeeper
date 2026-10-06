#!/usr/bin/env python3
import sys
import os
import re
import json
from datetime import datetime

def parse_hey_log(filepath):
    if not os.path.exists(filepath):
        return {"error": "File not found"}

    with open(filepath, 'r') as f:
        content = f.read()

    data = {
        "requests_per_sec": 0.0,
        "avg_latency_ms": 0.0,
        "fastest_ms": 0.0,
        "slowest_ms": 0.0,
        "status_codes": {}
    }

    rps_match = re.search(r'Requests/sec:\s*([0-9.]+)', content)
    if rps_match:
        data["requests_per_sec"] = float(rps_match.group(1))

    avg_match = re.search(r'Average:\s*([0-9.]+)\s*secs', content)
    if avg_match:
        data["avg_latency_ms"] = float(avg_match.group(1)) * 1000.0

    fast_match = re.search(r'Fastest:\s*([0-9.]+)\s*secs', content)
    if fast_match:
        data["fastest_ms"] = float(fast_match.group(1)) * 1000.0

    slow_match = re.search(r'Slowest:\s*([0-9.]+)\s*secs', content)
    if slow_match:
        data["slowest_ms"] = float(slow_match.group(1)) * 1000.0

    status_section = re.search(r'Status code distribution:\s*((?:\s*\[\d+\]\s*\d+\s*responses\n?)+)', content)
    if status_section:
        status_lines = status_section.group(1).strip().split('\n')
        for line in status_lines:
            code_match = re.search(r'\[(\d+)\]\s*(\d+)\s*responses', line)
            if code_match:
                code = int(code_match.group(1))
                count = int(code_match.group(2))
                data["status_codes"][code] = count

    return data
def get_cpu_cores():
    return os.cpu_count() or 1

def get_cpu_model():
    try:
        with open('/proc/cpuinfo', 'r') as f:
            for line in f:
                if line.strip().startswith('model name') or line.strip().startswith('Processor'):
                    return line.split(':', 1)[1].strip()
    except Exception:
        pass
    return "Unknown CPU"

def get_total_ram_gb():
    try:
        with open('/proc/meminfo', 'r') as f:
            for line in f:
                if line.startswith('MemTotal:'):
                    kb = int(line.split()[1])
                    return f"{kb / (1024 * 1024):.1f} GB"
    except Exception:
        pass
    return "Unknown RAM"

def get_kernel_version():
    try:
        with open('/proc/version', 'r') as f:
            return f.read().strip().split()[2]
    except Exception:
        pass
    return "Unknown Kernel"

def read_monitor_stats(filepath):
    if not os.path.exists(filepath):
        return {}
    with open(filepath, 'r') as f:
        return json.load(f)

def main():
    if len(sys.argv) < 2:
        print("Usage: report.py <output_directory>", file=sys.stderr)
        sys.exit(1)

    out_dir = sys.argv[1]
    
    test1_hey = parse_hey_log("/tmp/test1_hey.log")
    test1_sys = read_monitor_stats("/tmp/test1_stats.json")

    test2_hey = parse_hey_log("/tmp/test2_hey.log")
    test2_sys = read_monitor_stats("/tmp/test2_stats.json")

    test3_hey = parse_hey_log("/tmp/test3_hey.log")
    test3_sys = read_monitor_stats("/tmp/test3_stats.json")

    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    report_filename = f"10s-100con_{timestamp}.benchmark.md"
    report_path = os.path.join(out_dir, report_filename)

    def format_status(status_dict):
        if not status_dict:
            return "N/A"
        return ", ".join(f"{code}: {count}" for code, count in status_dict.items())

    # Retrieve host system metrics
    cores = get_cpu_cores()
    cpu_model = get_cpu_model()
    total_ram = get_total_ram_gb()
    kernel_version = get_kernel_version()

    # Build ASCII formatted values
    t1_status = format_status(test1_hey.get("status_codes"))
    t2_status = format_status(test2_hey.get("status_codes"))
    t3_status = format_status(test3_hey.get("status_codes"))

    t1_rps = f"{test1_hey.get('requests_per_sec', 0.0):,.2f}"
    t2_rps = f"{test2_hey.get('requests_per_sec', 0.0):,.2f}"
    t3_rps = f"{test3_hey.get('requests_per_sec', 0.0):,.2f}"

    t1_lat = f"{test1_hey.get('avg_latency_ms', 0.0):.2f} ms"
    t2_lat = f"{test2_hey.get('avg_latency_ms', 0.0):.2f} ms"
    t3_lat = f"{test3_hey.get('avg_latency_ms', 0.0):.2f} ms"

    t1_lat_range = f"{test1_hey.get('fastest_ms', 0.0):.2f} / {test1_hey.get('slowest_ms', 0.0):.2f} ms"
    t2_lat_range = f"{test2_hey.get('fastest_ms', 0.0):.2f} / {test2_hey.get('slowest_ms', 0.0):.2f} ms"
    t3_lat_range = f"{test3_hey.get('fastest_ms', 0.0):.2f} / {test3_hey.get('slowest_ms', 0.0):.2f} ms"

    t1_gk_cpu = "N/A"
    t2_gk_cpu = f"{test2_sys.get('gatekeeper', {}).get('cpu_avg', 0.0):.1f}% / {test2_sys.get('gatekeeper', {}).get('cpu_peak', 0.0):.1f}%"
    t3_gk_cpu = f"{test3_sys.get('gatekeeper', {}).get('cpu_avg', 0.0):.1f}% / {test3_sys.get('gatekeeper', {}).get('cpu_peak', 0.0):.1f}%"

    t1_gk_ram = "N/A"
    t2_gk_ram = f"{test2_sys.get('gatekeeper', {}).get('rss_avg', 0.0):.2f} MB / {test2_sys.get('gatekeeper', {}).get('rss_peak', 0.0):.2f} MB"
    t3_gk_ram = f"{test3_sys.get('gatekeeper', {}).get('rss_avg', 0.0):.2f} MB / {test3_sys.get('gatekeeper', {}).get('rss_peak', 0.0):.2f} MB"

    t1_ng_cpu = f"{test1_sys.get('nginx', {}).get('cpu_avg', 0.0):.1f}% / {test1_sys.get('nginx', {}).get('cpu_peak', 0.0):.1f}%"
    t2_ng_cpu = f"{test2_sys.get('nginx', {}).get('cpu_avg', 0.0):.1f}% / {test2_sys.get('nginx', {}).get('cpu_peak', 0.0):.1f}%"
    t3_ng_cpu = f"{test3_sys.get('nginx', {}).get('cpu_avg', 0.0):.1f}% / {test3_sys.get('nginx', {}).get('cpu_peak', 0.0):.1f}%"

    t1_ng_ram = f"{test1_sys.get('nginx', {}).get('rss_avg', 0.0):.2f} MB / {test1_sys.get('nginx', {}).get('rss_peak', 0.0):.2f} MB"
    t2_ng_ram = f"{test2_sys.get('nginx', {}).get('rss_avg', 0.0):.2f} MB / {test2_sys.get('nginx', {}).get('rss_peak', 0.0):.2f} MB"
    t3_ng_ram = f"{test3_sys.get('nginx', {}).get('rss_avg', 0.0):.2f} MB / {test3_sys.get('nginx', {}).get('rss_peak', 0.0):.2f} MB"

    def format_row(col1, col2, col3, col4):
        return f"| {col1:<32} | {col2:<28} | {col3:<28} | {col4:<28} |"

    divider = "+" + "-"*34 + "+" + "-"*30 + "+" + "-"*30 + "+" + "-"*30 + "+"

    ascii_table = "\n".join([
        divider,
        format_row("Metric", "Test 1: Baseline (Guardless)", "Test 2: Fast (No Cookie)", "Test 3: Slow (With Cookie)"),
        divider,
        format_row("Response Status", t1_status, t2_status, t3_status),
        format_row("Requests / Second (RPS)", t1_rps, t2_rps, t3_rps),
        format_row("Average Latency", t1_lat, t2_lat, t3_lat),
        format_row("Latency Range (Min / Max)", t1_lat_range, t2_lat_range, t3_lat_range),
        format_row("Gatekeeper CPU (Avg / Peak)", t1_gk_cpu, t2_gk_cpu, t3_gk_cpu),
        format_row("Gatekeeper RAM (Avg / Peak)", t1_gk_ram, t2_gk_ram, t3_gk_ram),
        format_row("Nginx CPU (Avg / Peak)", t1_ng_cpu, t2_ng_cpu, t3_ng_cpu),
        format_row("Nginx RAM (Avg / Peak)", t1_ng_ram, t2_ng_ram, t3_ng_ram),
        divider
    ])

    markdown = f"""# Gatekeeper Benchmark Results ({datetime.now().strftime("%Y-%m-%d %H:%M:%S")})

This document contains the performance test results of a synthetic load test against Gatekeeper running in Docker.

## System Configuration
- **Host CPU Model**: {cpu_model} ({cores} Cores)
- **Host Total RAM**: {total_ram}
- **Host Kernel**: {kernel_version}

## Container Configuration
- **CPU Limit**: 2.0 Cores (Simulated environment)
- **Memory Limit**: 2 GB RAM (Simulated environment)
- **Container OS**: Nginx (Debian bookworm)
- **Load Test Duration**: 10s
- **Client Concurrency**: 100 concurrent workers (unlimited requests)

## Benchmark Comparison

```text
{ascii_table}
```

## Explanations
1. **Test 1 (Baseline - Guardless)**: Measures raw Nginx performance without any authentication or proxying. Serves as a baseline reference.
2. **Test 2 (Gatekeeper Fast Path)**: Requests arrive without any cookie. Gatekeeper rejects them immediately without computing HMAC signatures since verification fails instantly with `401 Unauthorized`.
3. **Test 3 (Gatekeeper Slow Path)**: Requests contain a syntactically valid cookie with a future expiration date. This forces Gatekeeper to calculate the HMAC-SHA256 signature for each request and compare it with the cookie's hash before rejecting it.
"""

    with open(report_path, 'w') as f:
        f.write(markdown)

    print(f"Report successfully written to {report_path}")

if __name__ == '__main__':
    main()

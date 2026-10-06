#!/usr/bin/env python3
import time
import os
import sys
import argparse
import json
import signal

def get_cpu_cores():
    return os.cpu_count() or 1

def get_system_ticks():
    try:
        with open('/proc/stat', 'r') as f:
            line = f.readline()
            parts = line.split()
            return sum(int(x) for x in parts[1:])
    except IOError:
        return 0

def get_process_ticks(pid):
    try:
        with open(f'/proc/{pid}/stat', 'r') as f:
            parts = f.read().split()
            return int(parts[13]) + int(parts[14])
    except (IOError, IndexError):
        return None

def get_process_rss(pid):
    try:
        with open(f'/proc/{pid}/status', 'r') as f:
            for line in f:
                if line.startswith('VmRSS:'):
                    return int(line.split()[1]) # in KB
    except IOError:
        pass
    return None

def find_pids_by_name(name):
    pids = []
    for pid_str in os.listdir('/proc'):
        if pid_str.isdigit():
            try:
                with open(f'/proc/{pid_str}/comm', 'r') as f:
                    comm = f.read().strip()
                    if name in comm:
                        pids.append(int(pid_str))
            except IOError:
                pass
    return pids

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', required=True, help='Path to output JSON file')
    parser.add_argument('--interval', type=float, default=0.1, help='Sampling interval in seconds')
    args = parser.parse_args()

    gatekeeper_pids = find_pids_by_name('gatekeeper')
    nginx_pids = find_pids_by_name('nginx')

    if not gatekeeper_pids:
        print("Warning: No 'gatekeeper' process found.", file=sys.stderr)
    if not nginx_pids:
        print("Warning: No 'nginx' process found.", file=sys.stderr)

    cores = get_cpu_cores()
    history = []
    
    prev_sys_ticks = get_system_ticks()
    prev_proc_ticks = {}
    
    all_pids = gatekeeper_pids + nginx_pids
    for pid in all_pids:
        t = get_process_ticks(pid)
        if t is not None:
            prev_proc_ticks[pid] = t

    running = True

    def signal_handler(signum, frame):
        nonlocal running
        running = False

    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)

    print(f"Monitoring started. Gatekeeper PIDs: {gatekeeper_pids}, Nginx PIDs: {nginx_pids}. Cores: {cores}")

    while running:
        t_start = time.time()
        
        curr_sys_ticks = get_system_ticks()
        sys_diff = curr_sys_ticks - prev_sys_ticks
        prev_sys_ticks = curr_sys_ticks

        if sys_diff <= 0:
            sys_diff = 1

        gatekeeper_cpu = 0.0
        gatekeeper_rss = 0.0
        nginx_cpu = 0.0
        nginx_rss = 0.0

        for pid in gatekeeper_pids:
            curr_ticks = get_process_ticks(pid)
            rss = get_process_rss(pid)
            if curr_ticks is not None:
                if pid in prev_proc_ticks:
                    proc_diff = curr_ticks - prev_proc_ticks[pid]
                    cpu_usage = 100.0 * (proc_diff / sys_diff) * cores
                    gatekeeper_cpu += cpu_usage
                prev_proc_ticks[pid] = curr_ticks
            if rss is not None:
                gatekeeper_rss += rss / 1024.0

        for pid in nginx_pids:
            curr_ticks = get_process_ticks(pid)
            rss = get_process_rss(pid)
            if curr_ticks is not None:
                if pid in prev_proc_ticks:
                    proc_diff = curr_ticks - prev_proc_ticks[pid]
                    cpu_usage = 100.0 * (proc_diff / sys_diff) * cores
                    nginx_cpu += cpu_usage
                prev_proc_ticks[pid] = curr_ticks
            if rss is not None:
                nginx_rss += rss / 1024.0

        history.append({
            'timestamp': time.time(),
            'gatekeeper': {'cpu': gatekeeper_cpu, 'rss': gatekeeper_rss},
            'nginx': {'cpu': nginx_cpu, 'rss': nginx_rss}
        })

        elapsed = time.time() - t_start
        sleep_time = max(0.0, args.interval - elapsed)
        time.sleep(sleep_time)

    if history:
        gk_cpus = [h['gatekeeper']['cpu'] for h in history]
        gk_rsss = [h['gatekeeper']['rss'] for h in history]
        ngx_cpus = [h['nginx']['cpu'] for h in history]
        ngx_rsss = [h['nginx']['rss'] for h in history]

        summary = {
            'gatekeeper': {
                'cpu_avg': sum(gk_cpus) / len(gk_cpus) if gk_cpus else 0.0,
                'cpu_peak': max(gk_cpus) if gk_cpus else 0.0,
                'rss_avg': sum(gk_rsss) / len(gk_rsss) if gk_rsss else 0.0,
                'rss_peak': max(gk_rsss) if gk_rsss else 0.0,
            },
            'nginx': {
                'cpu_avg': sum(ngx_cpus) / len(ngx_cpus) if ngx_cpus else 0.0,
                'cpu_peak': max(ngx_cpus) if ngx_cpus else 0.0,
                'rss_avg': sum(ngx_rsss) / len(ngx_rsss) if ngx_rsss else 0.0,
                'rss_peak': max(ngx_rsss) if ngx_rsss else 0.0,
            }
        }

        with open(args.output, 'w') as f:
            json.dump(summary, f, indent=2)
        print(f"Summary written to {args.output}")

if __name__ == '__main__':
    main()

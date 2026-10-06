# Gatekeeper Benchmarking

This directory contains the tools to measure the resource utilization and throughput of Gatekeeper under synthetic load in an isolated Docker environment.

## Structure

```
benchmark/
  ├── compose.10s-100con.yml    # Docker Compose configuration for the 10s load test (100 concurrent)
  ├── build/
  │   └── 10s-100con/           # Contexts, scripts, and Dockerfile for this scenario
  │       ├── Dockerfile
  │       ├── nginx.conf
  │       ├── run_benchmarks.sh
  │       ├── monitor.py
  │       └── report.py
  └── results/                  # Benchmark results on the host (mounted)
```

## Prerequisites

- Docker and Docker Compose V2 installed.
- A compiled Gatekeeper release binary at `target/release/gatekeeper`. If not present, please build it on the host:
  ```bash
  cargo build --release
  ```

## Scenarios

So far we provide one test scenario: `10s-100con`. It basically runs a hey command with 100 concurrent clients for 10 seconds against an NGINX-Gatekeeper-Setup and logs the CPU/RAM/latency-metrics to show the system impact of gatekeeper.

## Running Benchmarks

Execute the following commands in the **root directory** of the project:

1. **Start the benchmark with Docker Compose**:
   This command automatically builds the image and runs the test suite. The results will be written directly to the mounted `results/` directory:
   ```bash
   sudo docker compose -f benchmark/compose.10s-100con.yml up --build
   ```

2. **Inspect the results**:
   After the tests complete, you will find the results at:
   `benchmark/results/10s-100con_YYYYMMDD_HHMMSS.benchmark.md`

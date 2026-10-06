FROM nginx:bookworm

# Install Python 3 for monitoring/reporting, and procps for ps
RUN apt-get update && apt-get install -y \
    python3 \
    procps \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Download static hey binary from official releases bucket
RUN curl -L -o /usr/local/bin/hey https://storage.googleapis.com/hey-releases/hey_linux_amd64 \
    && chmod +x /usr/local/bin/hey

# Create app directory structure
WORKDIR /app
RUN mkdir -p /app/services /results

# Copy configurations and binary from the host
COPY target/release/gatekeeper /app/gatekeeper
COPY config.toml /app/config.toml
COPY services/bot-whitelist.conf /app/services/bot-whitelist.conf

# Copy scenario assets directly to /app/
COPY benchmark/build/10s-100con/nginx.conf /app/nginx.conf
COPY benchmark/build/10s-100con/run_benchmarks.sh /app/run_benchmarks.sh
COPY benchmark/build/10s-100con/monitor.py /app/monitor.py
COPY benchmark/build/10s-100con/report.py /app/report.py

# Ensure all scripts are executable
RUN chmod +x /app/run_benchmarks.sh /app/monitor.py /app/report.py

ENTRYPOINT ["/app/run_benchmarks.sh"]

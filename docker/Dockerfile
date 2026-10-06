# Stage 1: Build daemon
FROM rust:bookworm AS builder
WORKDIR /usr/src/gatekeeper
# Pre-cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src
# Build actual application
COPY src ./src
COPY templates ./templates
RUN touch src/main.rs && cargo build --release

# Stage 2: Runtime
FROM nginx:bookworm

WORKDIR /app

#COPY target/release/gatekeeper /app/gatekeeper
COPY --from=builder /usr/src/gatekeeper/target/release/gatekeeper /app/gatekeeper
COPY docker/docker.config.toml /app/config.toml.example
COPY set-config.sh /app/set-config.sh
COPY docker/docker.theme.toml /app/theme.toml

# Copy scenario assets directly to /app/
COPY docker/nginx.conf /app/nginx.conf

COPY docker/run_showcase.sh /app/run_showcase.sh
RUN chmod +x /app/run_showcase.sh /app/gatekeeper

ENTRYPOINT ["/app/run_showcase.sh"]

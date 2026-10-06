# Setup for Local Development

*This guide assumes a Linux environment.*

## Prerequisites

If not already installed, install the Rust toolchain on your workstation/server.

### Install Rust (Linux / macOS)
Rust is most easily installed via `rustup`. Run the following command in your terminal:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
*Follow the on-screen instructions (select option `1` for the default installation).*  
After installation, reload your shell environment to make the commands available:
```bash
source "$HOME/.cargo/env"
```
Verify the installation with:
```bash
rustc --version
cargo --version
```

## Setup

### Prepare Repository
* Clone the Git repository.
* Navigate to the project directory:
  ```bash
  cd /opt/projects/gatekeeper/app
  ```

### Create Socket Directory & Set Permissions
Gatekeeper communicates over a Unix domain socket. For this socket file to be created, the target directory must exist and be writable by your current user. **Important:** Because `/var/run` is volatile (tmpfs), this socket directory disappears across system reboots. This script must therefore be rerun regularly. Alternatively, a service can be configured as in production, but this script has fewer side effects in local development.
```bash
sudo bash set-socket.sh
```

### Configure Settings
Copy the template configuration file. On first startup, a cryptographically secure key (`app_key`) is automatically generated and written to the file if this field is empty:
```bash
cp config.toml.example config.toml
```

### Start Gatekeeper
Start the service using Cargo. It downloads all dependencies, compiles the code, and starts the socket server:
```bash
cargo run -- KeyGenerate
cargo run
```

### Integrate NGINX
See the NGINX configuration in [docs/implementation/nginx/gatekeeper.conf.template](./implementation/nginx/gatekeeper.conf.template).
- Copy all snippets in `docs/implementation/nginx/snippets/gatekeeper` to `/etc/nginx/snippets/gatekeeper`
- In `/etc/nginx/nginx.conf`, add `include snippets/gatekeeper/upstream.conf;` before the sites-enabled statement (this defines the Gatekeeper upstream server)
- In the site configuration you want to protect, add the following inside the `server` block:
  - `include snippets/gatekeeper/endpoints.conf;`: defines the endpoints required by Gatekeeper (the verification form and cookie validation check)
  - `include snippets/gatekeeper/honeypot.conf;`: defines the scraper honeypot endpoint (optional)
  - Inside the location to protect (e.g., `/search`), add `include snippets/gatekeeper/protect.conf;` before the actual `proxy_pass`. This contains the `auth_request` directive and the redirect to the verification form.

### Notes for Further Development

Even minor changes to the HTML template strictly require a new `build`. Only changes to configuration files take effect without rebuilding.

## Build Release Binary

Compile the project in optimized release mode to minimize binary size and maximize execution speed:
```bash
cargo build --release
```
The compiled binary is located at:  
`target/release/gatekeeper`
# Integration

*This guide assumes a Linux environment.*

In production, the application runs as a performant, optimized background binary managed via a systemd service unit. 

## Setup for Production

### Build Release Binary (Optional)
If the binary still needs to be built (`cargo` must be installed):
```bash
cargo build --release
```
The compiled binary is located at:  
`target/release/gatekeeper`

> [!NOTE]
> The binaries related to releases can also be downloaded from [Github](https://github.com/telota/gatekeeper/releases)

### Create System User
Create a dedicated, unprivileged system user for the service:
```bash
sudo useradd -r -s /bin/false gatekeeper
```

### Prepare Directories and Adjust Permissions
* Create target directories, copy files, and set restrictive permissions:
  ```bash
  # Create directories
  sudo mkdir -p /etc/gatekeeper /usr/local/bin

  # Copy and secure binary (owned by root, executable by all, read-only)
  sudo cp target/release/gatekeeper /usr/local/bin/
  sudo chown root:root /usr/local/bin/gatekeeper
  sudo chmod 755 /usr/local/bin/gatekeeper
  ```
* Next, `config.toml` must be created. There are two options:  
  * Semi-automated solution:
    * Copy and run `set-config`
      ```bash
      sudo bash set-config.sh /etc/gatekeeper/config.toml
      ```
  * Manual solution:
    * Copy configuration
      ```bash
      sudo cp config.toml.example /etc/gatekeeper/config.toml
      cd /etc/gatekeeper/
      ```
    * Generate app key, for example:
      ```bash
      openssl rand -hex 32
      ```
      Then add the key to `config.toml` as `app_key`

* Set permissions  
  Owned by gatekeeper, readable only by gatekeeper:
  ```bash
  sudo chown -R gatekeeper:gatekeeper /etc/gatekeeper
  sudo chmod 600 /etc/gatekeeper/config.toml
  ```

### Create Systemd Service Unit
Copy the [service template](./implementation/system/gatekeeper.service) to `/etc/systemd/system/` and apply standard permissions for systemd services:
```bash
sudo cp docs/implementation/system/gatekeeper.service /etc/systemd/system/
sudo chown root:root /etc/systemd/system/gatekeeper.service
sudo chmod 644 /etc/systemd/system/gatekeeper.service
```

### Enable and Start Service
Reload systemd, enable autostart, and start Gatekeeper:
```bash
sudo systemctl daemon-reload
sudo systemctl enable gatekeeper
sudo systemctl start gatekeeper
```

Check service status:
```bash
sudo systemctl status gatekeeper
```

### Configure HTTPS in NGINX
In production, web traffic is encrypted via HTTPS.  
SSL detection occurs fully automatically: When NGINX forwards the encrypted TLS headers to Gatekeeper via the Unix socket, Gatekeeper automatically sets the cookie's `Secure` attribute to `true` (ensuring it is transmitted only over HTTPS).

Add the upstream connection and authentication proxying to your HTTPS server block (`listen 443 ssl;`) according to the [template](./implementation/nginx/gatekeeper.conf.template).



## Managing the System Service

Here are the most common commands for managing the Gatekeeper service on the server:

* **Show status:**
  ```bash
  sudo systemctl status gatekeeper
  ```
* **Start service:**
  ```bash
  sudo systemctl start gatekeeper
  ```
* **Stop service:**
  ```bash
  sudo systemctl stop gatekeeper
  ```
* **Restart service (e.g., after configuration changes):**
  ```bash
  sudo systemctl restart gatekeeper
  ```
* **Follow live logs:**
  ```bash
  sudo journalctl -u gatekeeper -f
  ```
* **Show the last 100 log lines:**
  ```bash
  sudo journalctl -u gatekeeper -n 100
  ```


## Zero-Downtime Updates

Thanks to the **fail-open mechanism** configured in NGINX (`error_page 502 504 =200 /gatekeeper-fallback;`), restarting or temporarily taking down Gatekeeper does not cause downtime for the protected web application. Requests pass through uninspected during this window.

To update Gatekeeper to a new version, follow these steps:

### Create Backup (Optional)
To quickly roll back to the old working version in case of an issue, we recommend not overwriting the binary directly:
```bash
sudo mv /usr/local/bin/gatekeeper /usr/local/bin/gatekeeper_bu
```
Note: Moving the file while running is safe because the active systemd process runs decoupled from the filesystem path.

### Deploy New Binary
*Replace `PATH` with the path to the binary*
```bash
sudo cp PATH/gatekeeper /usr/local/bin/
sudo chown root:root /usr/local/bin/gatekeeper
sudo chmod 755 /usr/local/bin/gatekeeper
```

### Restart Service
```bash
sudo systemctl restart gatekeeper
```
*Note: Traffic passes through unfiltered during the brief downtime, but the protected website remains accessible at all times.*

### Verify Successful Startup
```bash
sudo systemctl status gatekeeper
sudo journalctl -u gatekeeper -n 20
```
*In case of issues: Simply overwrite the new binary with the backup and restart the service.*


## Auxiliary Services (Bot Whitelisting & Dashboard)

The `services/` directory contains helper files for automated bot whitelisting and the internal dashboard:
* `crawler-feeds.conf`: Configuration file for search engine crawler IP ranges (e.g., Googlebot, Bingbot, DuckDuckBot).
* `helper.py`: A Python script executed hourly via cron job. It:
  1. Updates the IP whitelist (`services/bot-whitelist.conf`) based on the feeds in `crawler-feeds.conf`. If changes occur, it sends a `SIGHUP` signal to the Gatekeeper daemon to reload the configuration at runtime.
  2. Fetches the current daemon metrics and maintains a rolling 7-day history in `services/metrics_history.json`.
* `dashboard.html`: The client-side dashboard for visualizing activity.

### Production Setup & Permissions

In a production environment, the entire `services/` directory is placed in `/etc/gatekeeper/services`. Because the `helper.py` cron script requires write permissions to this directory (to update `bot-whitelist.conf` and `metrics_history.json`) and must communicate with the Unix socket, it should run under the unprivileged `gatekeeper` system user.

* **Move directories and set permissions:**
  ```bash
  # Copy directory
  sudo cp -r services /etc/gatekeeper/
  
  # Set ownership to gatekeeper
  sudo chown -R gatekeeper:gatekeeper /etc/gatekeeper/services
  
  # Restrict permissions (directory executable/readable, files writable/readable)
  sudo chmod 755 /etc/gatekeeper/services
  sudo chmod 644 /etc/gatekeeper/services/*
  sudo chmod 755 /etc/gatekeeper/services/helper.py
  ```

* **Set up hourly cron job for user `gatekeeper`:**
  Run the following command to register the cron job for the `gatekeeper` system user:
  ```bash
  (sudo crontab -u gatekeeper -l 2>/dev/null; echo "0 * * * * /usr/bin/python3 /etc/gatekeeper/services/helper.py") | sudo crontab -u gatekeeper -
  ```
  *Note:* The cron job runs hourly (at minute 0) and automatically updates the whitelist and metrics history.

* **Dashboard**
  The dashboard can be configured in NGINX. See [NGINX snippet](./implementation/nginx/snippets/gatekeeper/dashboard.conf).

## Verification Form Customization

```bash
sudo cp theme.toml.example theme.toml
```
* Set your color scheme
* Enter the strings and URLs for the available languages and remove the comments
* Restart the gatekeeper daemon
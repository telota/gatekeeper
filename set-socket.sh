#!/bin/bash

# For development only!

REAL_USER=${SUDO_USER:-$USER}
sudo mkdir -p /var/run/gatekeeper
sudo chown -R "$REAL_USER":www-data /var/run/gatekeeper

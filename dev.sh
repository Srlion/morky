#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
QUADLET_DIR="$HOME/.config/containers/systemd"

mkdir -p "$HOME/.local/share/morky/haproxy" "$QUADLET_DIR"

# Install quadlet files from repo
cp "$SCRIPT_DIR"/quadlet/* "$QUADLET_DIR/"
systemctl --user daemon-reload

# Build and restart
echo "Building image (debug)..."
podman build -t morky --build-arg CARGO_PROFILE=dev -f Dockerfile .

echo "Restarting service..."
systemctl --user restart morky

echo ""
echo "Running on http://localhost:9764"

if [[ "${1:-}" == "-f" ]]; then
    trap "systemctl --user stop morky; exit 0" INT TERM
    journalctl --user -u morky -f
else
    echo "Logs:  journalctl --user -u morky -f"
fi

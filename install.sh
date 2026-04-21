#!/usr/bin/env bash
set -euo pipefail

REPO="srlion/morky"
SERVICE_USER="${SUDO_USER:-$(whoami)}"
SERVICE_HOME=$(getent passwd "$SERVICE_USER" | cut -d: -f6)
SERVICE_UID=$(id -u "$SERVICE_USER")
BIN_DIR="$SERVICE_HOME/.local/bin"
QUADLET_DIR="$SERVICE_HOME/.config/containers/systemd"
DATA_DIR="$SERVICE_HOME/.local/share/morky"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
QUADLET_SRC="$SCRIPT_DIR/quadlet"
QUADLET_FILES=(morky.container morky-buildkit.volume morky-haproxy-net.network)

HAS_SUDO=false
[[ $EUID -eq 0 ]] || sudo -n true 2>/dev/null && HAS_SUDO=true

run() {
  local desc="$1"; shift
  echo -n "$desc... "
  local tmp; tmp=$(mktemp)
  if "$@" >"$tmp" 2>&1; then echo "✓"
  else echo "✗"; cat "$tmp"; rm -f "$tmp"; exit 1; fi
  rm -f "$tmp"
}

as_user() {
  if [[ "$(whoami)" == "$SERVICE_USER" ]]; then "$@"
  else sudo -u "$SERVICE_USER" -H "$@"; fi
}

ctl() { as_user bash -c "XDG_RUNTIME_DIR=/run/user/$SERVICE_UID systemctl --user $*"; }
is_installed() { as_user bash -c "command -v morky" &>/dev/null; }
is_active()    { ctl "is-active --quiet morky 2>/dev/null"; }

install_quadlets() {
  if [[ -d "$QUADLET_SRC" ]]; then
    as_user cp "$QUADLET_SRC"/* "$QUADLET_DIR/"
  else
    local base="https://raw.githubusercontent.com/$REPO/main/quadlet"
    for f in "${QUADLET_FILES[@]}"; do
      as_user curl -fsSL "$base/$f" -o "$QUADLET_DIR/$f"
    done
  fi
}

if ! is_installed && [[ "$HAS_SUDO" == false ]]; then
  echo "First install requires root. Run with sudo."
  exit 1
fi

if is_installed; then echo "Updating morky for user: $SERVICE_USER"
else echo "Installing morky for user: $SERVICE_USER"; fi

# first install or full update
if [[ "$HAS_SUDO" == true ]]; then
  run "podman" bash -c \
    'curl -fsSL https://github.com/srlion/podman-static/raw/main/install.sh -o /tmp/install.sh && bash /tmp/install.sh'

  run "unprivileged port 80" bash -c "
    echo 'net.ipv4.ip_unprivileged_port_start=80' | sudo tee /etc/sysctl.d/99-morky.conf >/dev/null
    sudo sysctl -q --system
  "

  run "user lingering" sudo loginctl enable-linger "$SERVICE_USER"
  run "podman socket" ctl "enable --now podman.socket"

  # migrate old system service
  if [[ -f /etc/systemd/system/morky.service ]]; then
    run "removing old system service" bash -c "
      sudo systemctl disable --now morky 2>/dev/null || true
      sudo rm -f /etc/systemd/system/morky.service
      sudo systemctl daemon-reload
    "
  fi
else
  echo "(no sudo - skipping podman, sysctl, linger)"
fi

as_user mkdir -p "$BIN_DIR" "$DATA_DIR/haproxy" "$QUADLET_DIR"

run "morky binary" as_user bash -c "
  LATEST=\$(curl -s https://api.github.com/repos/$REPO/releases/latest | grep tag_name | cut -d'\"' -f4)
  curl -fsSL \"https://github.com/$REPO/releases/download/\$LATEST/morky-linux-amd64\" -o /tmp/morky-new
  chmod +x /tmp/morky-new
  mv /tmp/morky-new \"$BIN_DIR/morky\"
"

run "quadlet files" install_quadlets

run "daemon reload" ctl "daemon-reload"

if is_active; then
  run "restarting morky" ctl "restart morky"
else
  run "starting morky" ctl "enable --now morky"
fi

echo ""
echo "Done!"

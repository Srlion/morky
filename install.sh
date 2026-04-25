#!/usr/bin/env bash
set -euo pipefail

REPO="srlion/morky"
IMAGE="ghcr.io/srlion/morky"
SERVICE_USER="$(whoami)"
SERVICE_HOME="$HOME"
SERVICE_UID=$(id -u)
QUADLET_DIR="$SERVICE_HOME/.config/containers/systemd"
DATA_DIR="$SERVICE_HOME/.local/share/morky"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
QUADLET_SRC="$SCRIPT_DIR/quadlet"
QUADLET_FILES=(morky.container morky-buildkit.volume morky-haproxy-net.network)

RESTORE_FILE=""

usage() {
    echo "Usage: $0 [--restore <backup.tar.gz>]"
    exit 1
}

while [[ $# -gt 0 ]]; do
    case "$1" in
    --restore)
        [[ -n "${2:-}" ]] || usage
        RESTORE_FILE="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"
        shift 2
        ;;
    *)
        usage
        ;;
    esac
done

if [[ -n "$RESTORE_FILE" ]] && [[ ! -f "$RESTORE_FILE" ]]; then
    echo "Backup file not found: $RESTORE_FILE"
    exit 1
fi

HAS_SUDO=false
if [[ $EUID -eq 0 ]]; then
    HAS_SUDO=true
elif id -nG "$SERVICE_USER" | grep -qw sudo; then
    sudo true || {
        echo "sudo authentication failed."
        exit 1
    }
    HAS_SUDO=true
fi

run() {
    local desc="$1"
    shift
    echo -n "$desc... "
    local tmp
    tmp=$(mktemp)
    if "$@" >"$tmp" 2>&1; then
        echo "✓"
    else
        echo "✗"
        cat "$tmp"
        rm -f "$tmp"
        exit 1
    fi
    rm -f "$tmp"
}

ctl() { systemctl --user "$@"; }
is_active() { ctl is-active --quiet morky 2>/dev/null; }

is_installed() {
    [[ -f "$DATA_DIR/data/db-sqlite.db" ]]
}

install_quadlets() {
    if [[ -d "$QUADLET_SRC" ]]; then
        cp "$QUADLET_SRC"/* "$QUADLET_DIR/"
    else
        for f in "${QUADLET_FILES[@]}"; do
            curl -fsSL -H "Accept: application/vnd.github.v3.raw" \
                "https://api.github.com/repos/$REPO/contents/quadlet/$f" \
                -o "$QUADLET_DIR/$f"
        done
    fi
}

get_latest_tag() {
    curl -s --max-time 10 "https://api.github.com/repos/$REPO/releases/latest" | grep tag_name | cut -d'"' -f4
}

# restore: extract version from backup
if [[ -n "$RESTORE_FILE" ]]; then
    RESTORE_VERSION=$(tar -xzf "$RESTORE_FILE" -O version.txt 2>/dev/null) || {
        echo "Failed to read version.txt from backup"
        exit 1
    }

    if [[ -z "$RESTORE_VERSION" ]]; then
        echo "version.txt in backup is empty"
        exit 1
    fi

    echo "Restoring backup (version $RESTORE_VERSION)"

    if is_installed && is_active; then
        echo ""
        echo "WARNING: morky is currently running. Restoring will replace ALL existing data."
        read -rp "Continue? [y/N] " confirm </dev/tty
        [[ "$confirm" =~ ^[Yy]$ ]] || exit 0
    fi

    TAG="$RESTORE_VERSION"
else
    TAG=""
fi

FIRST_INSTALL=false
if ! is_installed; then
    FIRST_INSTALL=true
fi

if [[ "$FIRST_INSTALL" == true ]] && [[ "$HAS_SUDO" == false ]]; then
    echo "First install requires sudo. Add your user to the sudo group:"
    echo "  su -c 'usermod -aG sudo $SERVICE_USER'"
    echo "Then log out and back in, and re-run this script."
    exit 1
fi

if [[ -z "$TAG" ]]; then
    if [[ -d "$QUADLET_SRC" ]]; then
        TAG="dev"
    else
        TAG=$(get_latest_tag)
        TAG="${TAG#v}"
    fi
fi

if [[ "$FIRST_INSTALL" == false ]] && [[ -z "$RESTORE_FILE" ]]; then
    echo "Updating morky for user: $SERVICE_USER"
else
    echo "Installing morky for user: $SERVICE_USER"
fi

if [[ "$HAS_SUDO" == true ]]; then
    run "podman" bash -c 'curl -fsSL https://github.com/srlion/podman-static/raw/main/install.sh | bash'

    run "unprivileged port 80" bash -c "
        echo 'net.ipv4.ip_unprivileged_port_start=80' | sudo tee /etc/sysctl.d/99-morky.conf >/dev/null
        sudo sysctl -q --system
    "

    run "user lingering" sudo loginctl enable-linger "$SERVICE_USER"
    run "podman socket" ctl enable --now podman.socket
else
    echo "(no sudo - skipping podman, sysctl, linger)"
fi

mkdir -p "$DATA_DIR/haproxy" "$QUADLET_DIR"

install_quadlets

# patch quadlet to use the GHCR image + tag (production only)
if [[ ! -d "$QUADLET_SRC" ]]; then
    sed -i "s|^Image=.*|Image=$IMAGE:${TAG}|" "$QUADLET_DIR/morky.container"
fi

# First install: collect email and inject as env var for auto-setup
if [[ "$FIRST_INSTALL" == true ]] && [[ -z "$RESTORE_FILE" ]]; then
    read -rp "Enter your email: " MORKY_EMAIL </dev/tty
    sed -i "/^\[Container\]/a Environment=MORKY_ADMIN_EMAIL=${MORKY_EMAIL}" \
        "$QUADLET_DIR/morky.container"
fi

run "daemon reload" ctl daemon-reload

# restore: stop morky, restore data, then start
if [[ -n "$RESTORE_FILE" ]]; then
    if is_active; then
        run "stopping morky" ctl stop morky
    fi

    RESTORE_TMP=$(mktemp -d)
    trap "rm -rf '$RESTORE_TMP'" EXIT

    run "extracting backup" tar -xzf "$RESTORE_FILE" -C "$RESTORE_TMP"

    # restore database
    run "restoring database" cp "$RESTORE_TMP/database.db" "$DATA_DIR/database.db"

    # restore volumes
    if [[ -d "$RESTORE_TMP/volumes" ]]; then
        for app_dir in "$RESTORE_TMP/volumes"/*/; do
            [[ -d "$app_dir" ]] || continue
            for vol_tar in "$app_dir"*.tar.gz; do
                [[ -f "$vol_tar" ]] || continue
                vol_name=$(basename "$vol_tar" .tar.gz)
                podman volume create "$vol_name" 2>/dev/null || true
                run "restoring volume $vol_name" podman volume import "$vol_name" "$vol_tar"
            done
        done
    fi

    run "starting morky" ctl start morky
    echo ""
    echo "Done! morky $TAG restored from backup."
else
    if is_active; then
        run "restarting morky" ctl restart morky
    else
        run "starting morky" ctl start morky
    fi

    if [[ "$FIRST_INSTALL" == true ]]; then
        # Remove email from quadlet now that morky has started and read it
        sed -i '/^Environment=MORKY_ADMIN_EMAIL/d' \
            "$QUADLET_DIR/morky.container"
        ctl daemon-reload

        # Wait for morky to write the credentials file
        CREDS_FILE="$DATA_DIR/data/admin_credentials"
        for _ in $(seq 1 30); do
            [[ -f "$CREDS_FILE" ]] && break
            sleep 1
        done

        if [[ -f "$CREDS_FILE" ]]; then
            MORKY_EMAIL=$(sed -n '1p' "$CREDS_FILE")
            MORKY_PASSWORD=$(sed -n '2p' "$CREDS_FILE")
            rm -f "$CREDS_FILE"
            echo ""
            echo "Account created!"
            echo "  Email:    $MORKY_EMAIL"
            echo "  Password: $MORKY_PASSWORD"
        else
            echo ""
            echo "Warning: could not read credentials. Check logs:"
            echo "  journalctl --user -u morky -n 20"
        fi
    fi

    echo ""
    echo "Done! morky ${TAG} is running."
fi

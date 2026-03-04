#!/usr/bin/env bash
# Idempotent: append GitLab SSH host block using repo under ignored/ssh/.
# postCreateCommand runs with cwd = workspace root.
set -euo pipefail
WS="$(cd "$(dirname "$0")/.." && pwd)"
KEY="$WS/ignored/ssh/gitlab_konnex_ed25519"
MARKER="# knx-subtensor: git.konnex.world (auto, see .devcontainer/setup-ssh-konnex.sh)"

mkdir -p "$HOME/.ssh"
chmod 700 "$HOME/.ssh"

if [[ ! -f "$KEY" ]]; then
	echo "setup-ssh-konnex: skip (no private key at $KEY)"
	exit 0
fi

chmod 600 "$KEY" 2>/dev/null || true

touch "$HOME/.ssh/config"
if grep -qF "$MARKER" "$HOME/.ssh/config" 2>/dev/null; then
	echo "setup-ssh-konnex: already configured"
	exit 0
fi

{
	echo ""
	echo "$MARKER"
	echo "Host git.konnex.world"
	echo "  User git"
	echo "  IdentityFile $KEY"
	echo "  IdentitiesOnly yes"
	echo "  IdentityAgent none"
	echo "  ConnectTimeout 45"
	echo "  ServerAliveInterval 5"
	echo "  ServerAliveCountMax 20"
} >>"$HOME/.ssh/config"
chmod 600 "$HOME/.ssh/config"
echo "setup-ssh-konnex: wrote Host git.konnex.world -> $KEY"

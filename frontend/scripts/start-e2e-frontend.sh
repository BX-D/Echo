#!/usr/bin/env zsh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
FRONTEND_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$FRONTEND_DIR"
export VITE_WS_URL=ws://127.0.0.1:3002/ws

npm run build
npm run preview -- --host 127.0.0.1 --port 4173

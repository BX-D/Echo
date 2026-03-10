#!/usr/bin/env zsh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

set -a
source "$ROOT_DIR/.env"
set +a

cd "$ROOT_DIR/fear-engine"
export SERVER_HOST=127.0.0.1
export SERVER_PORT=3002
export FRONTEND_URL=http://127.0.0.1:4173
unset ANTHROPIC_API_KEY
unset OPENAI_API_KEY

cargo run -p fear-engine-server

#!/usr/bin/env bash
set -eo pipefail

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
BACKEND_DIR="$ROOT_DIR/fear-engine"
FRONTEND_DIR="$ROOT_DIR/frontend"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

BACKEND_PID=""
FRONTEND_PID=""

log()  { echo -e "${GREEN}[FEAR ENGINE]${NC} $1"; }
warn() { echo -e "${YELLOW}[FEAR ENGINE]${NC} $1"; }
err()  { echo -e "${RED}[FEAR ENGINE]${NC} $1"; }

kill_port() {
    local port=$1
    local pids
    pids=$(lsof -ti :"$port" 2>/dev/null || true)
    if [ -n "$pids" ]; then
        echo "$pids" | xargs kill -9 2>/dev/null || true
        sleep 0.5
    fi
}

cleanup() {
    log "Shutting down..."

    # Kill by PID (process groups).
    if [ -n "$BACKEND_PID" ]; then
        kill -- -"$BACKEND_PID" 2>/dev/null || kill "$BACKEND_PID" 2>/dev/null || true
    fi
    if [ -n "$FRONTEND_PID" ]; then
        kill -- -"$FRONTEND_PID" 2>/dev/null || kill "$FRONTEND_PID" 2>/dev/null || true
    fi

    # Also kill by port in case process-group kill missed anything.
    kill_port "${SERVER_PORT:-3001}"
    kill_port 5173

    wait 2>/dev/null || true
    log "Stopped."
}
trap cleanup EXIT INT TERM

# ── Check prerequisites ──────────────────────────────────────────────────

log "Checking prerequisites..."

if ! command -v cargo &>/dev/null; then
    err "Rust/Cargo not found. Install from https://rustup.rs"
    exit 1
fi

if ! command -v node &>/dev/null; then
    err "Node.js not found. Install from https://nodejs.org"
    exit 1
fi

log "  Rust:  $(cargo --version)"
log "  Node:  $(node --version)"
log "  npm:   $(npm --version)"

# ── .env setup ───────────────────────────────────────────────────────────

if [ ! -f "$ROOT_DIR/.env" ]; then
    if [ -f "$ROOT_DIR/.env.example" ]; then
        cp "$ROOT_DIR/.env.example" "$ROOT_DIR/.env"
        warn "Created .env from .env.example — edit it to add your API keys."
    fi
fi

if [ -f "$ROOT_DIR/.env" ]; then
    set -a
    # shellcheck disable=SC1091
    source "$ROOT_DIR/.env"
    set +a
fi

# ── Install frontend dependencies ────────────────────────────────────────

FRONTEND_LOCKFILE="$FRONTEND_DIR/node_modules/.package-lock.json"
NEEDS_FRONTEND_INSTALL=false

if [ ! -d "$FRONTEND_DIR/node_modules" ]; then
    NEEDS_FRONTEND_INSTALL=true
elif [ ! -f "$FRONTEND_LOCKFILE" ]; then
    NEEDS_FRONTEND_INSTALL=true
elif [ "$FRONTEND_DIR/package.json" -nt "$FRONTEND_LOCKFILE" ] || [ "$FRONTEND_DIR/package-lock.json" -nt "$FRONTEND_LOCKFILE" ]; then
    NEEDS_FRONTEND_INSTALL=true
elif ! (cd "$FRONTEND_DIR" && npm ls >/dev/null 2>&1); then
    NEEDS_FRONTEND_INSTALL=true
fi

if [ "$NEEDS_FRONTEND_INSTALL" = true ]; then
    log "Installing frontend dependencies..."
    (cd "$FRONTEND_DIR" && npm install)
else
    log "Frontend dependencies already installed."
fi

# ── Build backend (force recompile) ───────────────────────────────────────

log "Cleaning and rebuilding backend..."
rm -f "$BACKEND_DIR/target/release/fear-engine-server"
(cd "$BACKEND_DIR" && \
    touch crates/server/src/main.rs crates/server/src/ws/handler.rs crates/server/src/game_loop.rs crates/storage/src/session.rs 2>/dev/null; \
    cargo build --release -p fear-engine-server 2>&1 | tail -1)

# ── Kill stale processes on our ports ────────────────────────────────────

SERVER_HOST="${SERVER_HOST:-127.0.0.1}"
SERVER_PORT="${SERVER_PORT:-3001}"
FRONTEND_HOST="127.0.0.1"
FRONTEND_PORT=5173
FRONTEND_WS_URL="ws://${SERVER_HOST}:${SERVER_PORT}/ws"

kill_port "$SERVER_PORT"
kill_port "$FRONTEND_PORT"

# ── Start backend (run the binary directly, not via cargo run) ───────────

BINARY="$BACKEND_DIR/target/release/fear-engine-server"
if [ ! -f "$BINARY" ]; then
    err "Binary not found at $BINARY"
    exit 1
fi

log "Starting backend on ${CYAN}http://${SERVER_HOST}:${SERVER_PORT}${NC}..."

SERVER_HOST="$SERVER_HOST" \
SERVER_PORT="$SERVER_PORT" \
FRONTEND_URL="http://${FRONTEND_HOST}:${FRONTEND_PORT}" \
DATABASE_URL="${DATABASE_URL:-sqlite://$BACKEND_DIR/fear_engine.db}" \
RUST_LOG="${RUST_LOG:-fear_engine=debug}" \
"$BINARY" &
BACKEND_PID=$!

# Wait for backend to be ready
log "Waiting for backend..."
for i in $(seq 1 30); do
    if curl -s "http://${SERVER_HOST}:${SERVER_PORT}/health" >/dev/null 2>&1; then
        log "Backend ready (PID $BACKEND_PID)."
        break
    fi
    if [ "$i" -eq 30 ]; then
        err "Backend failed to start within 30 seconds."
        exit 1
    fi
    sleep 1
done

# ── Start frontend ───────────────────────────────────────────────────────

log "Building frontend for production preview..."
(cd "$FRONTEND_DIR" && VITE_WS_URL="$FRONTEND_WS_URL" npm run build)

log "Starting frontend on ${CYAN}http://${FRONTEND_HOST}:${FRONTEND_PORT}${NC}..."

cd "$FRONTEND_DIR"
VITE_WS_URL="$FRONTEND_WS_URL" npm run preview -- --host "$FRONTEND_HOST" --port "$FRONTEND_PORT" &
FRONTEND_PID=$!
cd "$ROOT_DIR"

sleep 2

echo ""
echo -e "${GREEN}════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  It Learns Your Fear — Fear Engine is running${NC}"
echo -e "${GREEN}════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "  Frontend:  ${CYAN}http://${FRONTEND_HOST}:${FRONTEND_PORT}${NC}"
echo -e "  Backend:   ${CYAN}http://${SERVER_HOST}:${SERVER_PORT}${NC}"
echo -e "  Health:    ${CYAN}http://${SERVER_HOST}:${SERVER_PORT}/health${NC}"
echo -e "  WS URL:    ${CYAN}${FRONTEND_WS_URL}${NC}"
echo ""
echo -e "  Press ${YELLOW}Ctrl+C${NC} to stop all services."
echo ""

# Keep running until interrupted
wait

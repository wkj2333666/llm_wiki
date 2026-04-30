#!/bin/bash
# LLM Wiki Development Mode - Start both frontend and backend

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Check if debug server binary exists, build if not
if [ ! -f "$SCRIPT_DIR/src-server/target/debug/llm-wiki-server" ]; then
    echo "Debug server not found, building..."
    cd "$SCRIPT_DIR/src-server" && cargo build
fi

echo "Starting LLM Wiki Development Mode"
echo "  Frontend: vite dev server (port 1420)"
echo "  Backend:  debug server (port 3000)"
echo ""

# Kill any existing processes on exit
trap 'kill $(jobs -p) 2>/dev/null' EXIT

# Start backend in background
export LLM_WIKI_CONFIG="${LLM_WIKI_CONFIG:-$SCRIPT_DIR/server.toml}"
"$SCRIPT_DIR/src-server/target/debug/llm-wiki-server" &
echo "Backend started (PID: $!)"

# Wait a moment for backend to start
sleep 1

# Start frontend (this will run in foreground)
echo "Starting frontend..."
cd "$SCRIPT_DIR"
npm run dev
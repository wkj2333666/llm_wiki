#!/bin/bash
# LLM Wiki Development Mode - Start both frontend and backend

set -e

# Get project root directory (scripts/ is one level below root)
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Check if debug server binary exists, build if not
if [ ! -f "$PROJECT_ROOT/src-server/target/debug/llm-wiki-server" ]; then
    echo "Debug server not found, building..."
    cd "$PROJECT_ROOT/src-server" && cargo build
fi

echo "Starting LLM Wiki Development Mode"
echo "  Frontend: vite dev server (port 1420)"
echo "  Backend:  debug server (port 3000)"
echo ""

# Kill any existing processes on exit
trap 'kill $(jobs -p) 2>/dev/null' EXIT

# Start backend in background
export LLM_WIKI_CONFIG="${LLM_WIKI_CONFIG:-$PROJECT_ROOT/server.toml}"
"$PROJECT_ROOT/src-server/target/debug/llm-wiki-server" &
echo "Backend started (PID: $!)"

# Wait a moment for backend to start
sleep 1

# Start frontend (this will run in foreground)
echo "Starting frontend..."
cd "$PROJECT_ROOT"
npm run dev
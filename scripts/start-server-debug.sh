#!/bin/bash
# LLM Wiki Web Server Startup Script (Debug mode - faster build)

set -e

# Get project root directory (scripts/ is one level below root)
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Default config file path
CONFIG_FILE="${LLM_WIKI_CONFIG:-$PROJECT_ROOT/server.toml}"

# Check if config file exists
if [ ! -f "$CONFIG_FILE" ]; then
    echo "Warning: Config file not found at $CONFIG_FILE"
    echo "Creating default config..."
    cat > "$CONFIG_FILE" << 'EOF'
# LLM Wiki Web Server Configuration

[server]
port = 3000
token = ""
data_dir = "~/.llm-wiki-server"
static_dir = "dist-server/public"
projects_dir = "~/wiki-projects"

[llm]
provider = "openai"
url = "https://api.openai.com/v1"
api_key = ""
model = "gpt-4o"
max_context_size = 128000
api_mode = "chat_completions"

[embedding]
provider = "openai"
url = "https://api.openai.com/v1"
api_key = ""
model = "text-embedding-3-small"

[search]
enabled = true
provider = "duckduckgo"
api_key = ""
EOF
    echo "Created default config. Edit $CONFIG_FILE to configure your LLM provider."
fi

# Set config file path for server
export LLM_WIKI_CONFIG="$CONFIG_FILE"

echo "LLM Wiki Web Server (Debug)"
echo "  Config file:    $CONFIG_FILE"
echo "  Server binary:  $PROJECT_ROOT/src-server/target/debug/llm-wiki-server"
echo ""

# Start the server
exec "$PROJECT_ROOT/src-server/target/debug/llm-wiki-server"
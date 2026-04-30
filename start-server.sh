#!/bin/bash
# LLM Wiki Web Server Startup Script

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Default config file path
CONFIG_FILE="${LLM_WIKI_CONFIG:-$SCRIPT_DIR/server.toml}"

# Check if config file exists
if [ ! -f "$CONFIG_FILE" ]; then
    echo "Warning: Config file not found at $CONFIG_FILE"
    echo "Creating default config..."
    cat > "$CONFIG_FILE" << 'EOF'
# LLM Wiki Web Server Configuration

[server]
port = 3000
token = "your-secret-token"
data_dir = "~/.llm-wiki-server"
static_dir = "dist-server/public"

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
enabled = false
provider = "google"
api_key = ""
EOF
    echo "Created default config. Edit $CONFIG_FILE to configure your LLM provider."
fi

# Set config file path for server
export LLM_WIKI_CONFIG="$CONFIG_FILE"

echo "LLM Wiki Web Server"
echo "  Config file:    $CONFIG_FILE"
echo "  Server binary:  $SCRIPT_DIR/src-server/target/release/llm-wiki-server"
echo ""

# Start the server
exec "$SCRIPT_DIR/src-server/target/release/llm-wiki-server"

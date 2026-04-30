use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::env;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub port: u16,
    pub token: String,
    pub data_dir: PathBuf,
    pub static_dir: PathBuf,
    pub projects_dir: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LlmConfig {
    pub provider: String,
    pub url: String,
    pub api_key: String,
    pub model: String,
    pub max_context_size: usize,
    pub api_mode: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmbeddingConfig {
    pub provider: String,
    pub url: String,
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchConfig {
    pub enabled: bool,
    pub provider: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub llm: LlmConfig,
    pub embedding: EmbeddingConfig,
    pub search: SearchConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            token: String::new(),
            data_dir: dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("llm-wiki-server"),
            static_dir: PathBuf::from("dist-server/public"),
            projects_dir: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("wiki-projects"),
        }
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            url: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            model: "gpt-4o".to_string(),
            max_context_size: 128000,
            api_mode: "chat_completions".to_string(),
        }
    }
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            url: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            model: "text-embedding-3-small".to_string(),
        }
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "google".to_string(),
            api_key: String::new(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            llm: LlmConfig::default(),
            embedding: EmbeddingConfig::default(),
            search: SearchConfig::default(),
        }
    }
}

impl AppConfig {
    /// Load configuration from file with environment variable overrides
    pub fn load() -> Self {
        // Find config file
        let config_path = env::var("LLM_WIKI_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                // Look for server.toml in current directory and parent
                let cwd = std::env::current_dir().unwrap_or_default();
                if cwd.join("server.toml").exists() {
                    cwd.join("server.toml")
                } else if cwd.parent().map(|p| p.join("server.toml").exists()) == Some(true) {
                    cwd.parent().unwrap().join("server.toml")
                } else {
                    PathBuf::from("server.toml")
                }
            });

        let mut config = if config_path.exists() {
            tracing::info!("Loading config from {:?}", config_path);
            let content = std::fs::read_to_string(&config_path)
                .expect("Failed to read config file");
            toml::from_str(&content)
                .expect("Failed to parse config file")
        } else {
            tracing::warn!("Config file {:?} not found, using defaults + env vars", config_path);
            Self::default()
        };

        // Apply environment variable overrides
        if let Ok(token) = env::var("LLM_WIKI_TOKEN") {
            config.server.token = token;
        }
        if let Ok(port) = env::var("LLM_WIKI_PORT") {
            config.server.port = port.parse().unwrap_or(3000);
        }
        if let Ok(data_dir) = env::var("LLM_WIKI_DATA_DIR") {
            config.server.data_dir = PathBuf::from(data_dir);
        }
        if let Ok(static_dir) = env::var("LLM_WIKI_STATIC_DIR") {
            config.server.static_dir = PathBuf::from(static_dir);
        }

        // LLM overrides
        if let Ok(provider) = env::var("LLM_PROVIDER") {
            config.llm.provider = provider;
        }
        if let Ok(url) = env::var("LLM_URL") {
            config.llm.url = url;
        }
        if let Ok(api_key) = env::var("LLM_API_KEY") {
            config.llm.api_key = api_key;
        }
        if let Ok(model) = env::var("LLM_MODEL") {
            config.llm.model = model;
        }

        // Expand ~ in paths
        config.server.data_dir = expand_home(&config.server.data_dir);
        config.server.static_dir = expand_home(&config.server.static_dir);
        config.server.projects_dir = expand_home(&config.server.projects_dir);

        // Validate required fields
        if config.server.token.is_empty() {
            tracing::warn!("No auth token configured. API will be unprotected!");
        }

        config
    }

    /// Get LLM config as JSON for frontend
    pub fn llm_config_json(&self) -> serde_json::Value {
        // Determine provider type based on URL
        // If URL is not the default OpenAI URL, treat as custom endpoint
        let is_default_openai = self.llm.url == "https://api.openai.com/v1";
        let effective_provider = if self.llm.provider == "openai" && !is_default_openai {
            "custom"  // Custom OpenAI-compatible endpoint
        } else {
            &self.llm.provider
        };

        serde_json::json!({
            "provider": effective_provider,
            "apiKey": self.llm.api_key,
            "model": self.llm.model,
            "maxContextSize": self.llm.max_context_size,
            "ollamaUrl": if self.llm.provider == "ollama" { self.llm.url.clone() } else { "".to_string() },
            "customEndpoint": if effective_provider == "custom" || self.llm.provider == "custom" { self.llm.url.clone() } else { "".to_string() },
            "apiMode": self.llm.api_mode,
        })
    }

    /// Get embedding config as JSON for frontend
    pub fn embedding_config_json(&self) -> serde_json::Value {
        serde_json::json!({
            "provider": self.embedding.provider,
            "apiKey": self.embedding.api_key,
            "model": self.embedding.model,
        })
    }
}

fn expand_home(path: &PathBuf) -> PathBuf {
    if path.starts_with("~") {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let rest = path.strip_prefix("~").unwrap();
        home.join(rest)
    } else {
        path.clone()
    }
}
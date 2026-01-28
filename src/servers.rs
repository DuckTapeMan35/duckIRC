use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub name: String,
    pub address: String,
    pub port: u16,
    #[serde(default = "default_use_tls")]
    pub use_tls: bool,
}

fn default_use_tls() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ServerConfig {
    pub servers: Vec<Server>,
}

impl ServerConfig {
    pub fn load(path: &str) -> Result<Self> {
        let path = Path::new(path);
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // If file doesn't exist, create with defaults
        if !path.exists() {
            let default_config = Self::default_config();
            default_config.save(path.to_str().unwrap())?;
            return Ok(default_config);
        }
        
        let contents = fs::read_to_string(path)?;
        let config: ServerConfig = toml::from_str(&contents)?;
        Ok(config)
    }
    
    pub fn save(&self, path: &str) -> Result<()> {
        let toml_string = toml::to_string_pretty(self)?;
        fs::write(path, toml_string)?;
        Ok(())
    }
    
    pub fn default_config() -> Self {
        Self {
            servers: vec![
                Server {
                    name: "Libera".to_string(),
                    address: "irc.libera.chat".to_string(),
                    port: 6697,
                    use_tls: true,
                },
                Server {
                    name: "OFTC".to_string(),
                    address: "irc.oftc.net".to_string(),
                    port: 6697,
                    use_tls: true,
                },
            ],
        }
    }
    
    pub fn add_server(&mut self, name: String, address: String, port: u16, use_tls: bool) -> bool {
        // check if server with same name exists
        if self.servers.iter().any(|s| s.name == name) {
            return false;
        }
        self.servers.push(Server {
            name,
            address,
            port,
            use_tls,
        });
        true
    }
    
    pub fn remove_server(&mut self, name: &str) -> bool {
        if let Some(pos) = self.servers.iter().position(|s| s.name == name) {
            self.servers.remove(pos);
            true
        } else {
            false
        }
    }
    
    pub fn get_server(&self, name: &str) -> Option<&Server> {
        self.servers.iter().find(|s| s.name == name)
    }
    
    pub fn list_servers(&self) -> Vec<String> {
        self.servers
            .iter()
            .map(|s| format!("{}: {}:{}", s.name, s.address, s.port))
            .collect()
    }
}

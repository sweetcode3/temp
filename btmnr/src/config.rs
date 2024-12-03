use notify::{Watcher, RecursiveMode, watcher};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::{Arc, RwLock};
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub inactivity_timeout: u64,
    pub auto_connect: bool,
    pub device_address: String,
}

#[derive(Clone)]
pub struct ConfigManager {
    current_config: Arc<RwLock<Config>>,
    backup_config: Arc<RwLock<Config>>,
}

impl ConfigManager {
    pub fn new() -> Self {
        let config = Config::load().unwrap_or_default();
        let manager = ConfigManager {
            current_config: Arc::new(RwLock::new(config.clone())),
            backup_config: Arc::new(RwLock::new(config)),
        };
        manager.start_config_watcher();
        manager
    }

    pub fn get_config(&self) -> Config {
        self.current_config.read().unwrap().clone()
    }

    fn start_config_watcher(&self) {
        let config_clone = self.current_config.clone();
        let backup_clone = self.backup_config.clone();

        std::thread::spawn(move || {
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher = watcher(tx, Duration::from_secs(1)).unwrap();
            watcher.watch("config.json", RecursiveMode::NonRecursive).unwrap();

            loop {
                match rx.recv() {
                    Ok(_) => {
                        match Config::load() {
                            Ok(new_config) => {
                                let mut current = config_clone.write().unwrap();
                                let mut backup = backup_clone.write().unwrap();
                                *backup = current.clone();
                                *current = new_config;
                                log::info!("Configuration reloaded successfully");
                            }
                            Err(e) => {
                                log::error!("Failed to load new config: {}", e);
                                // Restore from backup
                                let backup = backup_clone.read().unwrap();
                                let mut current = config_clone.write().unwrap();
                                *current = backup.clone();
                                log::info!("Restored previous working configuration");
                            }
                        }
                    }
                    Err(e) => log::error!("Watch error: {:?}", e),
                }
            }
        });
    }
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_str = fs::read_to_string("config.json")
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        
        let config: Config = serde_json::from_str(&config_str)
            .map_err(|e| format!("Failed to parse config: {}", e))?;
        
        config.validate()?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.validate()?;
        let config_str = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        
        fs::write("config.json", config_str)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
        
        Ok(())
    }

    fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.inactivity_timeout == 0 {
            return Err("inactivity_timeout must be greater than 0".into());
        }

        if !self.device_address.contains(':') || self.device_address.len() != 17 {
            return Err("Invalid device address format".into());
        }

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            inactivity_timeout: 300,
            auto_connect: true,
            device_address: String::from("XX:XX:XX:XX:XX:XX"),
        }
    }
}

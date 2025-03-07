pub use crate::tracker::TrackerMode;
use config::{Config, ConfigError, File};
use serde::{Deserialize, Serialize, Serializer};
use std;
use std::collections::HashMap;
use std::fs;
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;
use toml;

#[derive(Serialize, Deserialize, PartialEq)]
pub enum TrackerServer {
    UDP,
    HTTP,
}

#[derive(Serialize, Deserialize)]
pub struct UdpTrackerConfig {
    pub enabled: bool,
    pub bind_address: String,
}

#[derive(Serialize, Deserialize)]
pub struct HttpTrackerConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub ssl_enabled: bool,
    pub ssl_bind_address: String,
    #[serde(serialize_with = "none_as_empty_string")]
    pub ssl_cert_path: Option<String>,
    #[serde(serialize_with = "none_as_empty_string")]
    pub ssl_key_path: Option<String>,
}

impl HttpTrackerConfig {
    pub fn is_ssl_enabled(&self) -> bool {
        self.ssl_enabled && self.ssl_cert_path.is_some() && self.ssl_key_path.is_some()
    }
}

#[derive(Serialize, Deserialize)]
pub struct HttpApiConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub access_tokens: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct Configuration {
    pub log_level: Option<String>,
    pub mode: TrackerMode,
    pub db_path: String,
    pub persistence: bool,
    pub cleanup_interval: Option<u64>,
    pub cleanup_peerless: bool,
    pub external_ip: Option<String>,
    pub announce_interval: u32,
    pub announce_interval_min: u32,
    pub peer_timeout: u32,
    pub on_reverse_proxy: bool,
    pub udp_trackers: Vec<UdpTrackerConfig>,
    pub http_trackers: Vec<HttpTrackerConfig>,
    pub http_api: HttpApiConfig,
}

#[derive(Debug)]
pub enum ConfigurationError {
    IOError(std::io::Error),
    ParseError(toml::de::Error),
    TrackerModeIncompatible,
}

impl std::fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ConfigurationError::IOError(e) => e.fmt(f),
            ConfigurationError::ParseError(e) => e.fmt(f),
            _ => write!(f, "{:?}", self),
        }
    }
}

impl std::error::Error for ConfigurationError {}

pub fn none_as_empty_string<T, S>(option: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    S: Serializer,
{
    if let Some(value) = option {
        value.serialize(serializer)
    } else {
        "".serialize(serializer)
    }
}

impl Configuration {
    pub fn load(data: &[u8]) -> Result<Configuration, toml::de::Error> {
        toml::from_slice(data)
    }

    pub fn load_file(path: &str) -> Result<Configuration, ConfigurationError> {
        match std::fs::read(path) {
            Err(e) => Err(ConfigurationError::IOError(e)),
            Ok(data) => match Self::load(data.as_slice()) {
                Ok(cfg) => Ok(cfg),
                Err(e) => Err(ConfigurationError::ParseError(e)),
            },
        }
    }

    pub fn get_ext_ip(&self) -> Option<IpAddr> {
        match &self.external_ip {
            None => None,
            Some(external_ip) => match IpAddr::from_str(external_ip) {
                Ok(external_ip) => Some(external_ip),
                Err(_) => None,
            },
        }
    }
}

impl Configuration {
    pub fn default() -> Configuration {
        let mut configuration = Configuration {
            log_level: Option::from(String::from("info")),
            mode: TrackerMode::PublicMode,
            db_path: String::from("data.db"),
            persistence: false,
            cleanup_interval: Some(600),
            cleanup_peerless: true,
            external_ip: Some(String::from("0.0.0.0")),
            announce_interval: 120,
            announce_interval_min: 120,
            peer_timeout: 900,
            on_reverse_proxy: false,
            udp_trackers: Vec::new(),
            http_trackers: Vec::new(),
            http_api: HttpApiConfig {
                enabled: true,
                bind_address: String::from("127.0.0.1:1212"),
                access_tokens: [(String::from("admin"), String::from("MyAccessToken"))]
                    .iter()
                    .cloned()
                    .collect(),
            },
        };
        configuration.udp_trackers.push(UdpTrackerConfig {
            enabled: false,
            bind_address: String::from("0.0.0.0:6969"),
        });
        configuration.http_trackers.push(HttpTrackerConfig {
            enabled: false,
            bind_address: String::from("0.0.0.0:6969"),
            ssl_enabled: false,
            ssl_bind_address: String::from("0.0.0.0:6868"),
            ssl_cert_path: None,
            ssl_key_path: None,
        });
        configuration
    }

    pub fn verify(&self) -> Result<(), ConfigurationError> {
        // UDP is not secure for sending private keys
        if self.mode == TrackerMode::PrivateMode {
            for udp_tracker in &self.udp_trackers {
                if udp_tracker.enabled {
                    return Err(ConfigurationError::TrackerModeIncompatible);
                }
            }
        }

        Ok(())
    }

    pub fn load_from_file() -> Result<Configuration, ConfigError> {
        let mut config = Config::new();
        const CONFIG_PATH: &str = "config.toml";

        config.merge(File::with_name(CONFIG_PATH))?;
        let config = config.try_into::<Configuration>()?;
        config.verify().unwrap();
        Ok(config)
    }
}

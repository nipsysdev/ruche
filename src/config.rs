use crate::utils::regex::PORT_REGEX;
use regex::Regex;
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer};
use tokio::fs::File;
use tokio::io::AsyncReadExt;

fn validate_port<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    struct RegexVisitor(&'static str);

    impl RegexVisitor {
        fn new(pattern: &'static str) -> Self {
            Self(pattern)
        }
    }

    impl<'de> Visitor<'de> for RegexVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "a string matching pattern: {}", self.0)
        }

        fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
            let regex = Regex::new(self.0).map_err(|_| E::custom("Invalid regex pattern"))?;

            if regex.is_match(value) {
                Ok(value.to_string())
            } else {
                Err(E::custom(format!(
                    "Value '{}' doesn't match pattern: {}",
                    value, self.0
                )))
            }
        }
    }

    deserializer.deserialize_string(RegexVisitor::new(PORT_REGEX))
}

#[derive(Deserialize, Clone)]
pub struct Config {
    bee: Bee,
    network: Network,
    chains: Chains,
    storage: Storage,
}

impl Config {
    pub async fn parse() -> Self {
        let mut file = File::open("config.toml")
            .await
            .expect("Failed to open config file");
        let mut content = String::new();
        file.read_to_string(&mut content)
            .await
            .expect("Failed to read config file");
        toml::from_str(&content).expect("Failed to parse config file")
    }
}

#[derive(Deserialize, Clone)]
struct Bee {
    image: String,
    password_path: String,
    welcome_msg: String,
}

#[derive(Deserialize, Clone)]
struct Network {
    nat_addr: String,
    #[serde(deserialize_with = "validate_port")]
    api_port: String,
    #[serde(deserialize_with = "validate_port")]
    p2p_port: String,
}

#[derive(Deserialize, Clone)]
struct Chains {
    eth_rpc: String,
    gno_rpc: String,
}

#[derive(Deserialize, Clone)]
struct Storage {
    volumes_parent: String,
    volume_name: String,
    node_qty_per_volume: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parsing_of_valid_config() {
        let mock_config = r#"
            [bee]
            image = "ethersphere/bee:2.3.2"
            password_path = "/var/lib/bee/password"
            welcome_msg = "Hello, Swarm!"

            [network]
            nat_addr = "1.1.1.1"
            api_port = "17xx"
            p2p_port = "18xx"

            [chains]
            eth_rpc = "https://some.rpc"
            gno_rpc = "https://some.rpc"

            [storage]
            volumes_parent = "/media"
            volume_name = "swarm_data_xx"
            node_qty_per_volume = 4
        "#;

        let config: Config = toml::from_str(mock_config).unwrap();

        assert_eq!(config.bee.image, "ethersphere/bee:2.3.2");
        assert_eq!(config.bee.password_path, "/var/lib/bee/password");
        assert_eq!(config.bee.welcome_msg, "Hello, Swarm!");

        assert_eq!(config.network.nat_addr, "1.1.1.1");
        assert_eq!(config.network.api_port, "17xx");
        assert_eq!(config.network.p2p_port, "18xx");

        assert_eq!(config.chains.eth_rpc, "https://some.rpc");
        assert_eq!(config.chains.gno_rpc, "https://some.rpc");

        assert_eq!(config.storage.volumes_parent, "/media");
        assert_eq!(config.storage.volume_name, "swarm_data_xx");
        assert_eq!(config.storage.node_qty_per_volume, 4);
    }

    #[tokio::test]
    async fn test_parsing_of_valid_network_conf() {
        let mock_config = r#"
            nat_addr = "1.1.1.1"
            api_port = "17xx"
            p2p_port = "18xx"
        "#;

        let network_conf: Network = toml::from_str(mock_config).unwrap();

        assert_eq!(network_conf.api_port, "17xx");
        assert_eq!(network_conf.p2p_port, "18xx");
    }

    #[tokio::test]
    async fn test_failure_of_parsing_invalid_api_port() {
        let mock_config = r#"
            nat_addr = "1.1.1.1"
            api_port = "1781"
            p2p_port = "18xx"
        "#;

        let result: Result<Network, _> = toml::from_str(mock_config);

        assert!(result.is_err());

        // Optional: Check specific error message
        if let Err(e) = result {
            assert!(e.to_string().contains("doesn't match pattern"));
        }
    }

    #[tokio::test]
    async fn test_failure_of_parsing_invalid_p2p_port() {
        let mock_config = r#"
            nat_addr = "1.1.1.1"
            api_port = "17xx"
            p2p_port = "1801"
        "#;

        let result: Result<Network, _> = toml::from_str(mock_config);

        assert!(result.is_err());

        // Optional: Check specific error message
        if let Err(e) = result {
            assert!(e.to_string().contains("doesn't match pattern"));
        }
    }
}

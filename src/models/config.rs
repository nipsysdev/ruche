use crate::utils::regex::{RegexVisitor, PORT_REGEX, VOLUME_NAME_REGEX};
use serde::{Deserialize, Deserializer};
use tokio::fs::File;
use tokio::io::AsyncReadExt;

fn validate_port<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_string(RegexVisitor::new(PORT_REGEX))
}

fn validate_volume_name<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_string(RegexVisitor::new(VOLUME_NAME_REGEX))
}

#[derive(Deserialize, Default, Clone)]
pub struct Config {
    pub bee: Bee,
    pub network: Network,
    pub chains: Chains,
    pub storage: Storage,
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

#[derive(Deserialize, Default, Clone)]
pub struct Bee {
    pub image: String,
    pub password_path: String,
    pub welcome_msg: String,
}

#[derive(Deserialize, Default, Clone)]
pub struct Network {
    pub nat_addr: String,
    #[serde(deserialize_with = "validate_port")]
    pub api_port: String,
    #[serde(deserialize_with = "validate_port")]
    pub p2p_port: String,
}

#[derive(Deserialize, Default, Clone)]
pub struct Chains {
    pub eth_rpc: String,
    pub gno_rpc: String,
}

#[derive(Deserialize, Default, Clone)]
pub struct Storage {
    pub volumes_parent: String,
    #[serde(deserialize_with = "validate_volume_name")]
    pub volume_name: String,
    pub node_qty_per_volume: u8,
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

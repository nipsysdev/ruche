use anyhow::{anyhow, Result};
use regex::Regex;

use crate::{models::config::Config, utils::regex::PORT_REGEX};

use super::bee_fn::format_id;

pub fn get_port(id: u8, base_port: &str) -> Result<String> {
    let re = Regex::new(PORT_REGEX)?;
    if !re.is_match(base_port) {
        return Err(anyhow!("Invalid base port '{}'", base_port));
    }

    Ok(base_port.replace("xx", &format_id(id)))
}

pub fn get_api_port(config: &Config, id: u8) -> Result<String> {
    return get_port(id, &config.network.api_port);
}

pub fn get_p2p_port(config: &Config, id: u8) -> Result<String> {
    return get_port(id, &config.network.p2p_port);
}

mod tests {
    use super::*;

    #[tokio::test]
    async fn should_return_port_from_id_and_base_port() {
        let id = 5;
        let base_port = "17xx";
        let expected_port = "1705";

        let port = get_port(id, base_port).unwrap();

        assert_eq!(port, expected_port);
    }

    #[tokio::test]
    async fn should_fail_to_return_port_from_invalid_base_port() {
        assert!(get_port(5, "1705").is_err());
        assert!(get_port(5, "test").is_err());
        assert!(get_port(5, "1x70").is_err());
        assert!(get_port(5, "1xx0").is_err());
        assert!(get_port(5, "15340xx").is_err());
    }

    #[tokio::test]
    async fn should_return_api_port_from_config() {
        let config = Config {
            network: crate::models::config::Network {
                api_port: "17xx".to_string(),
                p2p_port: "18xx".to_string(),
                ..Default::default()
            },
            ..Config::default()
        };

        let api_port = get_api_port(&config, 5).unwrap();
        let p2p_port = get_p2p_port(&config, 5).unwrap();

        assert_eq!(api_port, "1705");
        assert_eq!(p2p_port, "1805");
    }

    #[tokio::test]
    async fn should_fail_api_port_with_invalid_base() {
        let config = Config {
            network: crate::models::config::Network {
                api_port: "1705".to_string(),
                ..Default::default()
            },
            ..Config::default()
        };

        assert!(get_api_port(&config, 5).is_err());
    }

    #[tokio::test]
    async fn should_fail_p2p_port_with_invalid_base() {
        let config = Config {
            network: crate::models::config::Network {
                p2p_port: "test".to_string(),
                ..Default::default()
            },
            ..Config::default()
        };

        assert!(get_p2p_port(&config, 5).is_err());
    }
}

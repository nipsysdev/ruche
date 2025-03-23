use anyhow::Result;
use async_trait::async_trait;
use bollard::{
    container::{Config as ContainerConfig, CreateContainerOptions},
    secret::{HostConfig, PortBinding, RestartPolicy, RestartPolicyNameEnum},
    Docker as BollarDocker,
};
use dyn_clone::DynClone;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use crate::models::{bee::BeeInfo, config::Config};

dyn_clone::clone_trait_object!(BeeDocker);

#[async_trait]
pub trait BeeDocker: DynClone + Send + Sync {
    async fn new_bee_container(&self, bee: &BeeInfo, config: &Config) -> Result<()>;
}

#[derive(Clone)]
pub struct Docker {
    docker: Arc<Mutex<BollarDocker>>,
}

impl Docker {
    pub fn new() -> Self {
        let docker =
            BollarDocker::connect_with_socket_defaults().expect("Failed to connect to docker");
        Docker {
            docker: Arc::new(Mutex::new(docker)),
        }
    }

    fn get_container_config(bee: &BeeInfo, config: &Config) -> ContainerConfig<String> {
        let data_dir_mount = format!("{}:{}", bee.data_dir.to_string_lossy(), "/home/bee/.bee");
        let mut port_binding = HashMap::new();
        port_binding.insert(
            format!("{}/tcp", bee.api_port),
            Some(vec![PortBinding {
                host_port: Some(bee.api_port.clone()),
                host_ip: Some("127.0.0.1".to_owned()),
            }]),
        );
        port_binding.insert(
            format!("{}/tcp", bee.p2p_port),
            Some(vec![PortBinding {
                host_port: Some(bee.p2p_port.clone()),
                host_ip: Some("0.0.0.0".to_owned()),
            }]),
        );

        let extra_hosts = match config.network.use_docker_host {
            false => None,
            true => Some(vec!["host.docker.internal:host-gateway".to_owned()]),
        };

        ContainerConfig {
            image: Some(bee.image.clone()),
            host_config: Some(HostConfig {
                binds: Some(vec![data_dir_mount]),
                port_bindings: Some(port_binding),
                restart_policy: Some(RestartPolicy {
                    name: Some(RestartPolicyNameEnum::ALWAYS),
                    maximum_retry_count: None,
                }),
                extra_hosts,
                ..HostConfig::default()
            }),
            env: Some(vec![
                format!("BEE_API_ADDR=127.0.0.1:{}", bee.api_port),
                format!("BEE_BLOCKCHAIN_RPC_ENDPOINT={}", config.chains.gno_rpc),
                format!("BEE_DATA_DIR={}", "/home/bee/.bee"),
                format!("BEE_FULL_NODE={}", bee.full_node),
                format!("BEE_NAT_ADDR={}:{}", config.network.nat_addr, bee.p2p_port),
                format!("BEE_P2P_ADDR=:{}", bee.p2p_port),
                format!("BEE_PASSWORD={}", config.bee.password),
                format!("BEE_RESERVE_CAPACITY_DOUBLING={}", bee.reserve_doubling),
                format!("BEE_RESOLVER_OPTIONS={}", config.chains.eth_rpc),
                format!("BEE_SWAP_ENABLE={}", bee.swap_enable),
                format!("BEE_TARGET_NEIGHBORHOOD={}", bee.neighborhood),
                format!("BEE_WELCOME_MESSAGE={}", config.bee.welcome_msg),
            ]),
            ..ContainerConfig::default()
        }
    }
}

#[async_trait]
impl BeeDocker for Docker {
    async fn new_bee_container(&self, bee: &BeeInfo, config: &Config) -> Result<()> {
        let docker = self.docker.lock().await;

        let config = Docker::get_container_config(bee, config);

        docker
            .create_container(
                Some(CreateContainerOptions {
                    name: bee.name.clone(),
                    platform: None,
                }),
                config,
            )
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::models::config::{Bee, Chains, Network, Storage};

    use super::*;
    use std::path::PathBuf;

    // Helper function to create test data
    fn create_test_data() -> (BeeInfo, Config) {
        let bee_info = BeeInfo {
            id: 1,
            name: "node_01".to_string(),
            image: "ethersphere/bee:2.5.0".to_string(),
            neighborhood: "1111101010".to_string(),
            full_node: true,
            swap_enable: true,
            reserve_doubling: false,
            data_dir: PathBuf::from("/home/lowkey/swarm_test/swarm_data_01/node_01"),
            api_port: "1701".to_string(),
            p2p_port: "1801".to_string(),
        };

        let config = Config {
            bee: Bee {
                image: "ethersphere/bee:2.5.0".to_string(),
                password: "some-password".to_string(),
                welcome_msg: "Hello, Swarm!".to_string(),
                // Below values should be unused in the tested method
                full_node: false,
                swap_enable: false,
                reserve_doubling: true,
            },
            network: Network {
                nat_addr: "1.1.1.1".to_string(),
                api_port: "17xx".to_string(),
                p2p_port: "18xx".to_string(),
                use_docker_host: true,
            },
            chains: Chains {
                eth_rpc: "https://some.rpc".to_string(),
                gno_rpc: "http://host.docker.internal:8545".to_string(),
            },
            storage: Storage {
                root_path: PathBuf::from("/media"),
                parent_dir_format: "swarm_data_xx".to_string(),
                parent_dir_capacity: 4,
            },
        };

        (bee_info, config)
    }

    #[test]
    fn test_container_image() {
        let (bee_info, config) = create_test_data();

        let container_config = Docker::get_container_config(&bee_info, &config);

        assert_eq!(container_config.image.unwrap(), "ethersphere/bee:2.5.0");
    }

    #[test]
    fn test_volume_bindings() {
        let (bee_info, config) = create_test_data();

        let container_config = Docker::get_container_config(&bee_info, &config);
        let binds = container_config
            .host_config
            .as_ref()
            .unwrap()
            .binds
            .as_ref()
            .unwrap();

        assert_eq!(
            binds[0],
            "/home/lowkey/swarm_test/swarm_data_01/node_01:/home/bee/.bee"
        );
    }

    #[test]
    fn test_api_port_binding() {
        let (bee_info, config) = create_test_data();
        let container_config = Docker::get_container_config(&bee_info, &config);
        let port_bindings = container_config
            .host_config
            .as_ref()
            .unwrap()
            .port_bindings
            .as_ref()
            .unwrap();

        let api_key = "1701/tcp";
        let api_binding = port_bindings.get(api_key).unwrap();

        assert_eq!(
            api_binding,
            &Some(vec![PortBinding {
                host_port: Some("1701".to_owned()),
                host_ip: Some("127.0.0.1".to_owned()),
            }])
        );
    }

    #[test]
    fn test_p2p_port_binding() {
        let (bee_info, config) = create_test_data();
        let container_config = Docker::get_container_config(&bee_info, &config);
        let port_bindings = container_config
            .host_config
            .as_ref()
            .unwrap()
            .port_bindings
            .as_ref()
            .unwrap();

        let p2p_key = "1801/tcp";
        let p2p_binding = port_bindings.get(p2p_key).unwrap();

        assert_eq!(
            p2p_binding,
            &Some(vec![PortBinding {
                host_port: Some("1801".to_owned()),
                host_ip: Some("0.0.0.0".to_owned()),
            }])
        );
    }

    #[test]
    fn test_environment_variables() {
        let (bee_info, config) = create_test_data();

        let container_config = Docker::get_container_config(&bee_info, &config);
        let env = container_config.env.as_ref().unwrap();

        assert!(env.contains(&"BEE_API_ADDR=127.0.0.1:1701".to_owned()));
        assert!(env
            .contains(&"BEE_BLOCKCHAIN_RPC_ENDPOINT=http://host.docker.internal:8545".to_owned()));
        assert!(env.contains(&"BEE_DATA_DIR=/home/bee/.bee".to_owned()));
        assert!(env.contains(&"BEE_FULL_NODE=true".to_owned()));
        assert!(env.contains(&"BEE_NAT_ADDR=1.1.1.1:1801".to_owned()));
        assert!(env.contains(&"BEE_P2P_ADDR=:1801".to_owned()));
        assert!(env.contains(&"BEE_PASSWORD=some-password".to_owned()));
        assert!(env.contains(&"BEE_RESERVE_CAPACITY_DOUBLING=false".to_owned()));
        assert!(env.contains(&"BEE_RESOLVER_OPTIONS=https://some.rpc".to_owned()));
        assert!(env.contains(&"BEE_SWAP_ENABLE=true".to_owned()));
        assert!(env.contains(&"BEE_TARGET_NEIGHBORHOOD=1111101010".to_owned()));
        assert!(env.contains(&"BEE_WELCOME_MESSAGE=Hello, Swarm!".to_owned()));
    }

    #[test]
    fn test_restart_policy() {
        let (bee_info, config) = create_test_data();

        let container_config = Docker::get_container_config(&bee_info, &config);
        let restart_policy = container_config
            .host_config
            .as_ref()
            .unwrap()
            .restart_policy
            .as_ref()
            .unwrap();

        assert_eq!(restart_policy.name, Some(RestartPolicyNameEnum::ALWAYS));
        assert!(restart_policy.maximum_retry_count.is_none());
    }

    #[test]
    fn test_extra_hosts() {
        let (bee_info, config) = create_test_data();

        let container_config = Docker::get_container_config(&bee_info, &config);
        let extra_hosts = container_config
            .host_config
            .as_ref()
            .unwrap()
            .extra_hosts
            .as_ref()
            .unwrap();

        assert_eq!(
            extra_hosts,
            &vec!["host.docker.internal:host-gateway".to_string()]
        );
    }

    #[test]
    fn test_extra_hosts_disabled() {
        let (bee_info, mut config) = create_test_data();
        config.network.use_docker_host = false;

        let container_config = Docker::get_container_config(&bee_info, &config);

        assert!(container_config
            .host_config
            .as_ref()
            .unwrap()
            .extra_hosts
            .is_none());
    }
}

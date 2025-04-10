use anyhow::Result;
use async_trait::async_trait;
use bollard::{
    container::{
        Config as ContainerConfig, CreateContainerOptions, LogsOptions, RemoveContainerOptions,
        StartContainerOptions, StopContainerOptions,
    },
    image::CreateImageOptions,
    secret::{HostConfig, PortBinding, RestartPolicy, RestartPolicyNameEnum},
    Docker as BollarDocker,
};
use dyn_clone::DynClone;
use futures_util::TryStreamExt;
use nix::unistd::{getgid, getuid};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use crate::models::{bee::BeeInfo, config::Config};

dyn_clone::clone_trait_object!(BeeDocker);

#[async_trait]
pub trait BeeDocker: DynClone + Send + Sync {
    async fn create_bee_container(&self, bee: &BeeInfo, config: &Config) -> Result<()>;
    async fn start_bee_container(&self, name: &str) -> Result<()>;
    async fn stop_bee_container(&self, name: &str) -> Result<()>;
    async fn remove_bee_container(&self, name: &str) -> Result<()>;
    async fn recreate_container(&self, bee: &BeeInfo, config: &Config) -> Result<()>;
    async fn get_bee_container_logs(&self, name: &str) -> Result<Vec<String>>;
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
        let bee_data_dir = "/home/bee/.bee";
        let data_dir_mount = format!("{}:{}", bee.data_dir.to_string_lossy(), bee_data_dir);

        let mut port_binding = HashMap::new();
        port_binding.insert(
            bee.api_port.clone(),
            Some(vec![PortBinding {
                host_port: Some(bee.api_port.clone()),
                host_ip: Some("127.0.0.1".to_owned()),
            }]),
        );
        port_binding.insert(
            bee.p2p_port.clone(),
            Some(vec![PortBinding {
                host_port: Some(bee.p2p_port.clone()),
                host_ip: Some("0.0.0.0".to_owned()),
            }]),
        );

        let mut exposed_ports = HashMap::new();
        exposed_ports.insert(bee.api_port.to_string(), HashMap::new());
        exposed_ports.insert(bee.p2p_port.to_string(), HashMap::new());

        let extra_hosts = match config.network.use_docker_host {
            false => None,
            true => Some(vec!["host.docker.internal:host-gateway".to_owned()]),
        };

        ContainerConfig {
            image: Some(bee.image.clone()),
            cmd: Some(vec!["start".to_owned()]),
            host_config: Some(HostConfig {
                binds: Some(vec![data_dir_mount]),
                port_bindings: Some(port_binding),
                restart_policy: Some(RestartPolicy {
                    name: Some(RestartPolicyNameEnum::UNLESS_STOPPED),
                    maximum_retry_count: None,
                }),
                extra_hosts,
                ..Default::default()
            }),
            exposed_ports: Some(exposed_ports),
            user: Some(format!("{}:{}", getuid(), getgid())),
            env: Some(vec![
                format!("BEE_API_ADDR=0.0.0.0:{}", bee.api_port),
                format!("BEE_BLOCKCHAIN_RPC_ENDPOINT={}", config.chains.gno_rpc),
                format!("BEE_DATA_DIR={}", bee_data_dir),
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
            ..Default::default()
        }
    }
}

#[async_trait]
impl BeeDocker for Docker {
    async fn create_bee_container(&self, bee: &BeeInfo, config: &Config) -> Result<()> {
        let docker = self.docker.lock().await;

        let container_config = Docker::get_container_config(bee, config);

        docker
            .create_image(
                Some(CreateImageOptions {
                    from_image: config.bee.image.to_owned(),
                    ..Default::default()
                }),
                None,
                None,
            )
            .try_collect::<Vec<_>>()
            .await?;

        docker
            .create_container(
                Some(CreateContainerOptions {
                    name: bee.name.clone(),
                    platform: None,
                }),
                container_config,
            )
            .await?;

        Ok(())
    }

    async fn start_bee_container(&self, name: &str) -> Result<()> {
        let docker = self.docker.lock().await;
        docker
            .start_container(name, None::<StartContainerOptions<String>>)
            .await
            .map_err(Into::into)
    }

    async fn stop_bee_container(&self, name: &str) -> Result<()> {
        let docker = self.docker.lock().await;
        docker
            .stop_container(name, None::<StopContainerOptions>)
            .await
            .map_err(Into::into)
    }

    async fn remove_bee_container(&self, name: &str) -> Result<()> {
        let docker = self.docker.lock().await;
        docker
            .remove_container(name, None::<RemoveContainerOptions>)
            .await
            .map_err(Into::into)
    }

    async fn recreate_container(&self, bee: &BeeInfo, config: &Config) -> Result<()> {
        self.stop_bee_container(&bee.name).await.unwrap_or_default();
        self.remove_bee_container(&bee.name)
            .await
            .unwrap_or_default();
        self.create_bee_container(bee, config).await?;
        Ok(())
    }

    async fn get_bee_container_logs(&self, name: &str) -> Result<Vec<String>> {
        let docker = self.docker.lock().await;
        let logs = docker
            .logs(
                name,
                Some(LogsOptions::<String> {
                    stdout: true,
                    stderr: true,
                    ..Default::default()
                }),
            )
            .try_collect::<Vec<_>>()
            .await?;

        Ok(logs
            .into_iter()
            .map(|log| String::from_utf8_lossy(&log.into_bytes()).into_owned())
            .collect())
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
            ..Default::default()
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

        let api_key = bee_info.api_port.clone();
        let api_binding = port_bindings.get(&api_key).unwrap();

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

        let p2p_key = bee_info.p2p_port.clone();
        let p2p_binding = port_bindings.get(&p2p_key).unwrap();

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

        assert!(env.contains(&"BEE_API_ADDR=0.0.0.0:1701".to_owned()));
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

        assert_eq!(
            restart_policy.name,
            Some(RestartPolicyNameEnum::UNLESS_STOPPED)
        );
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

    #[test]
    fn test_cmd() {
        let (bee_info, config) = create_test_data();
        let container_config = Docker::get_container_config(&bee_info, &config);
        assert_eq!(container_config.cmd, Some(vec!["start".to_string()]));
    }

    #[test]
    fn test_exposed_ports() {
        let (bee_info, config) = create_test_data();
        let container_config = Docker::get_container_config(&bee_info, &config);
        let exposed_ports = container_config.exposed_ports.as_ref().unwrap();
        assert!(exposed_ports.contains_key("1701"));
        assert!(exposed_ports.contains_key("1801"));
    }

    #[test]
    fn test_user() {
        let (bee_info, config) = create_test_data();
        let container_config = Docker::get_container_config(&bee_info, &config);
        let user = container_config.user.as_ref().unwrap();
        let parts: Vec<&str> = user.split(':').collect();
        assert_eq!(parts.len(), 2);
        assert!(parts[0].parse::<u32>().is_ok());
        assert!(parts[1].parse::<u32>().is_ok());
    }
}

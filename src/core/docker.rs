use anyhow::Result;
use async_trait::async_trait;
use bollard::{
    container::{Config, CreateContainerOptions},
    secret::HostConfig,
    Docker as BollarDocker,
};
use dyn_clone::DynClone;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use crate::models::bee::BeeInfo;

dyn_clone::clone_trait_object!(BeeDocker);

#[async_trait]
pub trait BeeDocker: DynClone + Send + Sync {
    async fn new_bee_container(&self, bee: BeeInfo) -> Result<()>;
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
}

#[async_trait]
impl BeeDocker for Docker {
    async fn new_bee_container(&self, bee: BeeInfo) -> Result<()> {
        let docker = self.docker.lock().await;

        let path_mount = format!("{}:{}", bee.data_dir.to_string_lossy(), "/home/bee/.bee");

        let options = Some(CreateContainerOptions {
            name: bee.name,
            platform: None,
        });

        let config = Config {
            image: Some(bee.image),
            host_config: Some(HostConfig {
                binds: Some(vec![path_mount]),
                ..Default::default()
            }),
            ..Config::default()
        };

        docker.create_container(options, config).await?;

        Ok(())
    }
}

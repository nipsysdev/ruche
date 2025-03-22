use crate::models::config::Config;
use bollard::Docker as BollarDocker;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Docker {
    docker: Arc<Mutex<BollarDocker>>,
    config: Config,
}

impl Docker {
    pub fn new(config: Config) -> Self {
        let docker =
            BollarDocker::connect_with_socket_defaults().expect("Failed to connect to docker");
        Docker {
            docker: Arc::new(Mutex::new(docker)),
            config,
        }
    }
}

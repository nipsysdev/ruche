use async_trait::async_trait;
use bollard::Docker as BollarDocker;
use dyn_clone::DynClone;
use std::sync::Arc;
use tokio::sync::Mutex;

dyn_clone::clone_trait_object!(BeeDocker);

#[async_trait]
pub trait BeeDocker: DynClone + Send + Sync {}

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

impl BeeDocker for Docker {}

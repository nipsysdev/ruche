use bollard::Docker;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct DockerService {
    docker: Arc<Mutex<Docker>>,
}

impl DockerService {
    pub fn new() -> Self {
        let docker = Docker::connect_with_socket_defaults().expect("Failed to connect to docker");
        DockerService {
            docker: Arc::new(Mutex::new(docker)),
        }
    }
}

mod bee_fn;
mod neighborhood_fn;
mod network_fn;
mod storage_fn;

use std::path::PathBuf;

use anyhow::Result;
use bee_fn::*;
use neighborhood_fn::*;
use storage_fn::*;

use crate::{
    core::{database::BeeDatabase, docker::BeeDocker},
    models::{
        bee::{BeeData, BeeInfo},
        config::Config,
    },
};

#[derive(Clone)]
pub struct BeeService {
    config: Config,
    db: Box<dyn BeeDatabase>,
    docker: Box<dyn BeeDocker>,
}

impl BeeService {
    pub fn new(config: Config, db: Box<dyn BeeDatabase>, docker: Box<dyn BeeDocker>) -> Self {
        BeeService { config, db, docker }
    }

    pub fn get_node_name(id: u8) -> String {
        get_node_name(id)
    }

    pub async fn get_neighborhood() -> Result<String> {
        get_neighborhood().await
    }

    pub async fn create_node_dir(&self, bee_id: u8) -> Result<PathBuf> {
        create_node_dir(&self.config, bee_id).await
    }

    pub async fn ensure_capacity(&self) -> Result<bool> {
        ensure_capacity(self.db.clone()).await
    }

    pub async fn get_new_bee_id(&self) -> Result<u8> {
        get_new_bee_id(self.db.clone()).await
    }

    pub fn new_bee_data(&self, id: u8, neighborhood: &str, data_dir: &PathBuf) -> BeeData {
        new_bee_data(&self.config, id, neighborhood, data_dir)
    }

    pub async fn save_bee(&self, bee_data: &BeeData) -> Result<()> {
        save_bee(self.db.clone(), bee_data).await
    }

    pub fn bee_data_to_info(&self, bee_data: &BeeData) -> Result<BeeInfo> {
        bee_data_to_info(&self.config, bee_data)
    }

    pub async fn get_bee(&self, bee_id: u8) -> Result<Option<BeeData>> {
        get_bee(self.db.clone(), bee_id).await
    }

    pub async fn get_bees(&self) -> Result<Vec<BeeData>> {
        get_bees(self.db.clone()).await
    }

    pub async fn count_bees(&self) -> Result<u64> {
        count_bees(self.db.clone()).await
    }

    pub async fn delete_bee(&self, bee_id: u8) -> Result<()> {
        delete_bee(&self.config, self.db.clone(), bee_id).await
    }

    pub async fn create_bee_container(&self, bee: &BeeInfo) -> Result<()> {
        create_bee_container(&self.config, self.docker.clone(), bee).await
    }

    pub async fn start_bee_container(&self, name: &str) -> Result<()> {
        start_bee_container(self.docker.clone(), name).await
    }

    pub async fn start_bee_containers(&self, names: Vec<String>) -> Result<()> {
        start_bee_containers(self.docker.clone(), names).await
    }

    pub async fn stop_bee_container(&self, name: &str) -> Result<()> {
        stop_bee_container(self.docker.clone(), name).await
    }

    pub async fn stop_bee_containers(&self, names: Vec<String>) -> Result<()> {
        stop_bee_containers(self.docker.clone(), names).await
    }

    pub async fn remove_bee_container(&self, name: &str) -> Result<()> {
        remove_bee_container(self.docker.clone(), name).await
    }

    pub async fn recreate_bee_container(&self, bee: &BeeInfo) -> Result<()> {
        recreate_bee_container(&self.config, self.docker.clone(), bee).await
    }

    pub async fn recreate_bee_containers(&self, bees: Vec<BeeInfo>) -> Result<()> {
        recreate_bee_containers(&self.config, self.docker.clone(), bees).await
    }

    pub async fn get_bee_container_logs(&self, name: &str) -> Result<Vec<String>> {
        get_bee_container_logs(self.docker.clone(), name).await
    }
}

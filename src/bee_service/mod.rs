mod bee_fn;
mod neighborhood_fn;
mod network_fn;
mod storage_fn;

use std::path::PathBuf;

use anyhow::Result;
use bee_fn::*;
use neighborhood_fn::*;
use network_fn::*;
use storage_fn::*;

use crate::{
    core::database::BeeDatabase,
    models::{
        bee::{BeeData, BeeInfo},
        config::Config,
    },
};

#[derive(Clone)]
pub struct BeeService {
    config: Config,
    db: Box<dyn BeeDatabase>,
}

impl BeeService {
    pub fn new(config: Config, db: Box<dyn BeeDatabase>) -> Self {
        BeeService { config, db }
    }

    pub fn format_id(id: u8) -> String {
        format_id(id)
    }

    pub fn get_node_name(id: u8) -> String {
        get_node_name(id)
    }

    pub fn get_port(id: u8, base_port: &str) -> Result<String> {
        get_port(id, base_port)
    }

    pub async fn get_neighborhood() -> Result<String> {
        get_neighborhood().await
    }

    pub fn get_api_port(&self, id: u8) -> Result<String> {
        get_api_port(&self.config, id)
    }

    pub fn get_p2p_port(&self, id: u8) -> Result<String> {
        get_p2p_port(&self.config, id)
    }

    pub fn get_dir_id(&self, bee_id: u8) -> u8 {
        get_dir_id(&self.config, bee_id)
    }

    pub fn get_parent_dir_name(&self, bee_id: u8) -> Result<String> {
        get_parent_dir_name(&self.config, bee_id)
    }

    pub fn get_node_path(&self, bee_id: u8) -> Result<PathBuf> {
        get_node_path(&self.config, bee_id)
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

    pub async fn save_bee(&self, bee_data: &BeeData) -> Result<BeeData> {
        save_bee(self.db.clone(), bee_data).await
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

    pub fn data_to_info(&self, data: &BeeData) -> Result<BeeInfo> {
        data_to_info(&self.config, data)
    }
}

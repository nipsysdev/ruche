use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::bee_service::BeeService;

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct BeeData {
    pub id: u8,
    pub neighborhood: String,
    pub full_node: bool,
    pub swap_enable: bool,
    pub reserve_doubling: bool,
    pub data_dir: PathBuf,
}

impl BeeData {
    pub fn name(&self) -> String {
        BeeService::get_node_name(self.id)
    }
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct BeeInfo {
    pub id: u8,
    pub name: String,
    pub image: String,
    pub neighborhood: String,
    pub full_node: bool,
    pub swap_enable: bool,
    pub reserve_doubling: bool,
    pub data_dir: PathBuf,
    pub api_port: String,
    pub p2p_port: String,
}

impl BeeInfo {
    pub fn new(data: &BeeData, image: &str, api_port: &str, p2p_port: &str) -> BeeInfo {
        BeeInfo {
            id: data.id,
            name: BeeService::get_node_name(data.id),
            image: image.to_owned(),
            neighborhood: data.neighborhood.to_owned(),
            full_node: data.full_node,
            swap_enable: data.swap_enable,
            reserve_doubling: data.reserve_doubling,
            data_dir: data.data_dir.to_owned(),
            api_port: api_port.to_owned(),
            p2p_port: p2p_port.to_owned(),
        }
    }
}

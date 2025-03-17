use crate::services::bee_service::BeeService;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct BeeData {
    pub id: u8,
    pub neighborhood: String,
    pub reserve_doubling: bool,
}

impl BeeData {
    pub fn name(&self) -> String {
        BeeService::get_name(self.id)
    }
}

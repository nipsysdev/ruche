use crate::services::db_service::DbService;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct Bee {
    id: u8,
    neighborhood: String,
    reserve_doubling: bool,
}

impl Bee {
    pub fn format_id(id: u8) -> String {
        format!("{:02}", id)
    }

    pub fn get_name(id: u8) -> String {
        format!("node_{}", Self::format_id(id))
    }

    pub async fn create(db: DbService) -> Result<Bee, String> {
        let count = db.count_bees().await?;
        if count >= 99 {
            // todo: error too many bees
        }
        let bees = db.get_bees().await?;
        let mut available_ids = (1..99).collect::<Vec<u8>>();
        for bee in bees.flatten() {
            available_ids.retain(|id| *id != bee.id);
        }
        let new_id = available_ids.first().ok_or("No id to use".to_string())?;

        // todo: continue implementation

        Ok(Bee {
            id: *new_id,
            neighborhood: String::new(),
            reserve_doubling: false,
        })
    }

    pub fn name(&self) -> String {
        format!("node_{}", Self::format_id(self.id))
    }
}

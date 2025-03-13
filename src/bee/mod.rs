use crate::services::db_service::BeeDatabase;
use crate::utils::regex::PORT_REGEX;
use anyhow::{anyhow, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct Bee {
    pub id: u8,
    pub neighborhood: String,
    pub reserve_doubling: bool,
}

impl Bee {
    pub fn format_id(id: u8) -> String {
        format!("{:02}", id)
    }

    pub fn get_name(id: u8) -> String {
        format!("node_{}", Self::format_id(id))
    }

    pub fn get_port(id: u8, base_port: &str) -> Result<String> {
        let re = Regex::new(PORT_REGEX)?;
        if !re.is_match(base_port) {
            return Err(anyhow!(""));
        }

        Ok(base_port.replace("xx", Self::format_id(id).as_str()))
    }

    pub async fn create(db: &dyn BeeDatabase) -> Result<Bee> {
        let count = db.count_bees().await?;
        if count >= 99 {
            return Err(anyhow!("max capacity reached"));
        }
        let bees = db.get_bees().await?;
        let mut available_ids = (1..99).collect::<Vec<u8>>();

        for bee in bees {
            available_ids.retain(|id| *id != bee.id);
        }

        let new_id = available_ids
            .first()
            .ok_or(anyhow::anyhow!("Unable to get new bee id"))?;

        // todo: continue implementation

        let bee = Bee {
            id: *new_id,
            neighborhood: String::new(),
            reserve_doubling: false,
        };
        db.add_bee(bee.clone()).await?;
        Ok(bee)
    }

    pub fn name(&self) -> String {
        Self::get_name(self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::db_service::MockDbService;

    #[tokio::test]
    async fn should_format_id() {
        assert_eq!(Bee::format_id(5), "05");
        assert_eq!(Bee::format_id(40), "40");
        assert_eq!(Bee::format_id(99), "99");
    }

    #[tokio::test]
    async fn should_return_name_from_id() {
        let bee = Bee {
            id: 5,
            neighborhood: String::new(),
            reserve_doubling: false,
        };

        assert_eq!(Bee::get_name(5), "node_05");
        assert_eq!(bee.name(), "node_05");
    }

    #[tokio::test]
    async fn should_return_port_from_id_and_base_port() {
        let id = 5;
        let base_port = "17xx";
        let expected_port = "1705";

        let port = Bee::get_port(id, base_port).unwrap();

        assert_eq!(port, expected_port);
    }

    #[tokio::test]
    async fn should_fail_to_return_port_from_invalid_base_port() {
        assert!(Bee::get_port(5, "1705").is_err());
        assert!(Bee::get_port(5, "test").is_err());
        assert!(Bee::get_port(5, "1x70").is_err());
        assert!(Bee::get_port(5, "1xx0").is_err());
        assert!(Bee::get_port(5, "15340xx").is_err());
    }

    #[tokio::test]
    async fn should_create_first_bee() {
        let mock = MockDbService::default();

        let new_bee = Bee::create(&mock).await.unwrap();

        assert_eq!(new_bee.id, 1);
        assert_eq!(mock.count_bees().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn should_add_additional_bee() {
        let mock = MockDbService::default();

        mock.add_bee(Bee {
            id: 1,
            neighborhood: String::new(),
            reserve_doubling: false,
        })
        .await
        .unwrap();

        mock.add_bee(Bee {
            id: 2,
            neighborhood: String::new(),
            reserve_doubling: false,
        })
        .await
        .unwrap();

        let new_bee = Bee::create(&mock).await.unwrap();

        assert_eq!(new_bee.id, 3);
        assert_eq!(mock.count_bees().await.unwrap(), 3);
    }

    #[tokio::test]
    async fn should_fail_creating_when_max_capacity_reached() {
        let mock = MockDbService::default();

        for id in 1..=99 {
            mock.add_bee(Bee {
                id,
                neighborhood: String::new(),
                reserve_doubling: false,
            })
            .await
            .unwrap();
        }

        let result = Bee::create(&mock).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn should_pick_first_available_id() {
        let mock = MockDbService::default();

        mock.add_bee(Bee {
            id: 1,
            neighborhood: String::new(),
            reserve_doubling: false,
        })
        .await
        .unwrap();
        mock.add_bee(Bee {
            id: 3,
            neighborhood: String::new(),
            reserve_doubling: false,
        })
        .await
        .unwrap();

        let new_bee = Bee::create(&mock).await.unwrap();

        assert_eq!(new_bee.id, 2);
        assert_eq!(mock.count_bees().await.unwrap(), 3);
    }
}

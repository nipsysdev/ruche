use crate::services::db_service::BeeDatabase;
use anyhow::Result;
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

    pub async fn create(db: &dyn BeeDatabase) -> Result<Bee> {
        let count = db.count_bees().await?;
        if count >= 99 {
            // todo: error too many bees
        }
        let bees = db.get_bees().await?;
        let mut available_ids = (1..99).collect::<Vec<u8>>();

        for bee in bees {
            available_ids.retain(|id| *id != bee.id);
        }

        let new_id = available_ids
            .first()
            .ok_or(anyhow::anyhow!("No id to use"))?;

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
        format!("node_{}", Self::format_id(self.id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::db_service::MockDbService;

    #[tokio::test]
    async fn test_format_id() {
        assert_eq!(Bee::format_id(5), "05");
        assert_eq!(Bee::format_id(99), "99"); // since u8 can't be over 255, but the function accepts any u8
    }

    #[tokio::test]
    async fn test_get_name() {
        let bee = Bee {
            id: 5,
            neighborhood: String::new(),
            reserve_doubling: false,
        };
        assert_eq!(bee.name(), "node_05");
    }

    #[tokio::test]
    async fn test_create_first_bee() {
        let mock = MockDbService::default();

        let new_bee = Bee::create(&mock).await.unwrap();
        assert_eq!(new_bee.id, 1);
        assert_eq!(mock.count_bees().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_create_with_existing_ids() {
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

    #[tokio::test]
    async fn test_create_no_available_id() {
        let mock = MockDbService::default();

        for id in 1..99 {
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
    async fn test_create_all_ids_taken() {
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
    async fn test_create_with_available_gap() {
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
        assert_eq!(new_bee.id, 2); // Should pick the first missing ID
        assert_eq!(mock.count_bees().await.unwrap(), 3);
    }
}

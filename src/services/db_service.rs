use crate::bee::Bee;
use anyhow::Error;
use anyhow::Result;
use async_trait::async_trait;
use polodb_core::bson::doc;
use polodb_core::{Collection, CollectionT, Database};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[async_trait]
pub trait BeeDatabase {
    async fn add_bee(&self, bee: Bee) -> Result<()>;
    async fn add_bees(&self, bees: Vec<Bee>) -> Result<()>;
    async fn count_bees(&self) -> Result<u64>;
    async fn get_bees(&self) -> Result<Vec<Bee>>;
}

#[derive(Clone)]
pub struct DbService {
    db: Arc<RwLock<Database>>,
}

impl DbService {
    pub fn new() -> Self {
        let db = Database::open_path("ruche.db").expect("Failed to open database");
        DbService {
            db: Arc::new(RwLock::new(db)),
        }
    }

    async fn get_bees_col_write(&self) -> Collection<Bee> {
        self.db.write().await.collection::<Bee>("bees")
    }

    async fn get_bees_col_read(&self) -> Collection<Bee> {
        self.db.read().await.collection::<Bee>("bees")
    }
}

#[async_trait]
impl BeeDatabase for DbService {
    async fn add_bee(&self, bee: Bee) -> Result<()> {
        let collection = self.get_bees_col_write().await;
        collection.insert_one(bee)?;
        Ok(())
    }

    async fn add_bees(&self, bees: Vec<Bee>) -> Result<()> {
        let collection = self.get_bees_col_write().await;
        collection.insert_many(bees)?;
        Ok(())
    }

    async fn count_bees(&self) -> Result<u64> {
        let collection = self.get_bees_col_read().await;
        collection.count_documents().map_err(Error::from)
    }

    async fn get_bees(&self) -> Result<Vec<Bee>> {
        let collection = self.get_bees_col_read().await;
        let cursor = collection
            .find(doc! {})
            .sort(doc! {
                "id": 1
            })
            .run()
            .map_err(Error::from)?;

        let mut bees = Vec::new();
        for result in cursor {
            let bee = result.map_err(Error::from)?;
            bees.push(bee);
        }
        Ok(bees)
    }
}

#[derive(Default, Clone)]
pub struct MockDbService {
    db: Arc<RwLock<VecDeque<Bee>>>,
}

impl MockDbService {
    async fn get_bees_col_write(&self) -> RwLockWriteGuard<'_, VecDeque<Bee>> {
        self.db.write().await
    }

    async fn get_bees_col_read(&self) -> RwLockReadGuard<'_, VecDeque<Bee>> {
        self.db.read().await
    }
}

#[async_trait]
impl BeeDatabase for MockDbService {
    async fn add_bee(&self, bee: Bee) -> Result<()> {
        let mut queue = self.get_bees_col_write().await;
        queue.push_back(bee);
        Ok(())
    }

    async fn add_bees(&self, bees: Vec<Bee>) -> Result<()> {
        let mut queue = self.get_bees_col_write().await;
        queue.extend(bees);
        Ok(())
    }

    async fn count_bees(&self) -> Result<u64> {
        let queue = self.get_bees_col_read().await;
        Ok(queue.len() as u64)
    }

    async fn get_bees(&self) -> Result<Vec<Bee>> {
        let queue = self.get_bees_col_read().await;
        Ok(queue.clone().make_contiguous().to_vec())
    }
}

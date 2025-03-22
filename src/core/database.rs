use anyhow::Error;
use anyhow::Result;
use async_trait::async_trait;
use dyn_clone::DynClone;
use polodb_core::bson::doc;
use polodb_core::Database as PoloDb;
use polodb_core::{Collection, CollectionT};
use std::collections::VecDeque;
use std::sync::Arc;

dyn_clone::clone_trait_object!(BeeDatabase);

#[async_trait]
pub trait BeeDatabase: DynClone + Send + Sync {
    async fn add_bee(&self, bee: BeeData) -> Result<()>;
    async fn add_bees(&self, bees: Vec<BeeData>) -> Result<()>;
    async fn count_bees(&self) -> Result<u64>;
    async fn get_bee(&self, bee_id: u8) -> Result<Option<BeeData>>;
    async fn get_bees(&self) -> Result<Vec<BeeData>>;
    async fn delete_bee(&self, bee_id: u8) -> Result<()>;
}

use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::models::bee::BeeData;

#[derive(Clone)]
pub struct Database {
    db: Arc<RwLock<PoloDb>>,
}

impl Database {
    pub fn new() -> Self {
        let db = PoloDb::open_path("ruche.db").expect("Failed to open database");
        Database {
            db: Arc::new(RwLock::new(db)),
        }
    }

    async fn get_bees_col_write(&self) -> Collection<BeeData> {
        self.db.write().await.collection::<BeeData>("bees")
    }

    async fn get_bees_col_read(&self) -> Collection<BeeData> {
        self.db.read().await.collection::<BeeData>("bees")
    }
}

#[async_trait]
impl BeeDatabase for Database {
    async fn add_bee(&self, bee: BeeData) -> Result<()> {
        let collection = self.get_bees_col_write().await;
        collection.insert_one(bee)?;
        Ok(())
    }

    async fn add_bees(&self, bees: Vec<BeeData>) -> Result<()> {
        let collection = self.get_bees_col_write().await;
        collection.insert_many(bees)?;
        Ok(())
    }

    async fn count_bees(&self) -> Result<u64> {
        let collection = self.get_bees_col_read().await;
        collection.count_documents().map_err(Error::from)
    }

    async fn get_bee(&self, bee_id: u8) -> Result<Option<BeeData>> {
        let collection = self.get_bees_col_read().await;
        let result = collection.find_one(doc! {"id": bee_id as i32})?;
        Ok(result)
    }

    async fn get_bees(&self) -> Result<Vec<BeeData>> {
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

    async fn delete_bee(&self, bee_id: u8) -> Result<()> {
        let collection = self.get_bees_col_write().await;
        collection.delete_one(doc! {"id": bee_id as i32})?;
        Ok(())
    }
}

#[derive(Default, Clone)]
pub struct MockDbService {
    db: Arc<RwLock<VecDeque<BeeData>>>,
}

impl MockDbService {
    async fn get_bees_col_write(&self) -> RwLockWriteGuard<'_, VecDeque<BeeData>> {
        self.db.write().await
    }

    async fn get_bees_col_read(&self) -> RwLockReadGuard<'_, VecDeque<BeeData>> {
        self.db.read().await
    }
}

#[async_trait]
impl BeeDatabase for MockDbService {
    async fn add_bee(&self, bee: BeeData) -> Result<()> {
        let mut queue = self.get_bees_col_write().await;
        queue.push_back(bee);
        Ok(())
    }

    async fn add_bees(&self, bees: Vec<BeeData>) -> Result<()> {
        let mut queue = self.get_bees_col_write().await;
        queue.extend(bees);
        Ok(())
    }

    async fn count_bees(&self) -> Result<u64> {
        let queue = self.get_bees_col_read().await;
        Ok(queue.len() as u64)
    }

    async fn get_bee(&self, bee_id: u8) -> Result<Option<BeeData>> {
        let queue = self.get_bees_col_read().await;
        Ok(queue.get(bee_id as usize).cloned())
    }

    async fn get_bees(&self) -> Result<Vec<BeeData>> {
        let queue = self.get_bees_col_read().await;
        Ok(queue.clone().make_contiguous().to_vec())
    }

    async fn delete_bee(&self, bee_id: u8) -> Result<()> {
        let mut queue = self.get_bees_col_write().await;
        queue.retain(|bee| bee.id != bee_id);
        Ok(())
    }
}

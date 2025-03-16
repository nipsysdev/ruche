use crate::models::BeeData;
use anyhow::Error;
use anyhow::Result;
use async_trait::async_trait;
use dyn_clone::DynClone;
use polodb_core::bson::doc;
use polodb_core::{ClientCursor, Collection, CollectionT, Database};
use std::sync::Arc;

#[async_trait]
pub trait BeeDatabase: DynClone + Send + Sync {
    async fn add_bee(&self, bee: BeeData) -> Result<()>;
    async fn add_bees(&self, bees: Vec<BeeData>) -> Result<()>;
    async fn count_bees(&self) -> Result<u64>;
    async fn get_bee(&self, bee_id: u8) -> Result<Option<BeeData>>;
    async fn get_bees(&self) -> Result<ClientCursor<BeeData>>;
    async fn delete_bee(&self, bee_id: u8) -> Result<()>;
}

use tokio::sync::RwLock;

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

    async fn get_bees_col_write(&self) -> Collection<BeeData> {
        self.db.write().await.collection::<BeeData>("bees")
    }

    async fn get_bees_col_read(&self) -> Collection<BeeData> {
        self.db.read().await.collection::<BeeData>("bees")
    }
}

#[async_trait]
impl BeeDatabase for DbService {
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

    async fn get_bees(&self) -> Result<ClientCursor<BeeData>> {
        let collection = self.get_bees_col_read().await;
        let cursor = collection
            .find(doc! {})
            .sort(doc! {
                "id": 1
            })
            .run()
            .map_err(Error::from)?;
        Ok(cursor)
    }

    async fn delete_bee(&self, bee_id: u8) -> Result<()> {
        let collection = self.get_bees_col_write().await;
        collection.delete_one(doc! {"id": bee_id as i32})?;
        Ok(())
    }
}

/*#[derive(Default, Clone)]
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

    async fn get_bees(&self) -> Result<Vec<BeeData>> {
        let queue = self.get_bees_col_read().await;
        Ok(queue.clone().make_contiguous().to_vec())
    }
}
*/

use crate::bee::Bee;
use polodb_core::bson::doc;
use polodb_core::{ClientCursor, Collection, CollectionT, Database};
use std::sync::Arc;
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

    async fn get_bees_col_write(&self) -> Collection<Bee> {
        self.db.write().await.collection::<Bee>("bees")
    }

    async fn get_bees_col_read(&self) -> Collection<Bee> {
        self.db.read().await.collection::<Bee>("bees")
    }

    pub async fn add_bee(&self, bee: Bee) -> Result<(), String> {
        let collection = self.get_bees_col_write().await;
        collection.insert_one(bee).map_err(|err| err.to_string())?;
        Ok(())
    }

    pub async fn add_bees(&self, bees: Vec<Bee>) -> Result<(), String> {
        let collection = self.get_bees_col_write().await;
        collection
            .insert_many(bees)
            .map_err(|err| err.to_string())?;
        Ok(())
    }

    pub async fn count_bees(&self) -> Result<u64, String> {
        let collection = self.get_bees_col_read().await;
        collection.count_documents().map_err(|err| err.to_string())
    }

    pub async fn get_bees(&self) -> Result<ClientCursor<Bee>, String> {
        let collection = self.get_bees_col_read().await;
        collection
            .find(doc! {})
            .sort(doc! {
                "id": 1
            })
            .run()
            .map_err(|err| err.to_string())
    }
}

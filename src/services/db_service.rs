use polodb_core::Database;
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
}

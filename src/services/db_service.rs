use std::sync::Arc;
use polodb_core::Database;

#[derive(Clone)]
pub struct DbService {
    db: Arc<Database>,
}

impl DbService {
    pub fn init() -> Self {
        DbService {
            db: Arc::new(Database::open_path("ruche.db").expect("Failed to open database"))
        }
    }
}

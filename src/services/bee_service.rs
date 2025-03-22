use crate::constants::NEIGHBORHOOD_API_URL;
use crate::models::bee::{BeeData, BeeInfo};
use crate::models::config::Config;
use crate::services::db_service::BeeDatabase;
use crate::utils::regex::{PORT_REGEX, VOLUME_NAME_REGEX};
use anyhow::{anyhow, Result};
use regex::Regex;
use std::env;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tokio::fs;

dyn_clone::clone_trait_object!(BeeDatabase);

#[derive(Clone)]
pub struct BeeService {
    config: Config,
    db: Box<dyn BeeDatabase>,
}

impl BeeService {
    pub fn new(config: Config, db: Box<dyn BeeDatabase>) -> Self {
        BeeService { config, db }
    }

    pub fn format_id(id: u8) -> String {
        format!("{:02}", id)
    }

    pub fn get_node_name(id: u8) -> String {
        format!("node_{}", Self::format_id(id))
    }

    pub fn get_port(id: u8, base_port: &str) -> Result<String> {
        let re = Regex::new(PORT_REGEX)?;
        if !re.is_match(base_port) {
            return Err(anyhow!("Invalid base port '{}'", base_port));
        }

        Ok(base_port.replace("xx", &Self::format_id(id)))
    }

    pub async fn get_neighborhood() -> Result<String> {
        let url =
            env::var("NEIGHBORHOOD_API_URL").unwrap_or_else(|_| NEIGHBORHOOD_API_URL.to_string());

        Ok(reqwest::get(url)
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?
            .get("neighborhood")
            .ok_or(anyhow!("Missing 'neighborhood' field"))?
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid 'neighborhood' field"))?
            .to_owned())
    }

    pub fn get_api_port(&self, id: u8) -> Result<String> {
        return Self::get_port(id, &self.config.network.api_port);
    }

    pub fn get_p2p_port(&self, id: u8) -> Result<String> {
        return Self::get_port(id, &self.config.network.p2p_port);
    }

    pub fn get_dir_id(&self, bee_id: u8) -> u8 {
        ((bee_id - 1) / self.config.storage.parent_dir_capacity) + 1
    }

    pub fn get_parent_dir_name(&self, bee_id: u8) -> Result<String> {
        let dir_name_format = &self.config.storage.parent_dir_format;

        let re = Regex::new(VOLUME_NAME_REGEX)?;
        if !re.is_match(dir_name_format) {
            return Err(anyhow!("Invalid parent name format '{}'", dir_name_format));
        }

        Ok(dir_name_format.replace("xx", &Self::format_id(self.get_dir_id(bee_id))))
    }

    pub fn get_node_path(&self, bee_id: u8) -> Result<PathBuf> {
        let root_path = &self.config.storage.root_path;
        let parent_name = self.get_parent_dir_name(bee_id)?;
        let parent_path = Path::new(root_path).join(parent_name);
        Ok(parent_path.join(BeeService::get_node_name(bee_id)))
    }

    pub async fn create_node_dir(&self, bee_id: u8) -> Result<PathBuf> {
        let node_path = self.get_node_path(bee_id)?;

        if node_path.exists() {
            return Err(anyhow!(
                "Directory '{}' already exists",
                node_path.display()
            ));
        }

        fs::create_dir_all(&node_path).await?;

        // Could it work without this?
        /*let bee_uid = User::from_name("bee")?
            .map(|user| user.uid)
            .ok_or(anyhow!("Missing bee user"))?;

        let systemd_journal_gid = Group::from_name("systemd-journal")?
            .map(|group| group.gid)
            .ok_or(anyhow!("Missing systemd-journal group"))?;

        chown(
            &dir_path,
            Some(u32::from(bee_uid)),
            Some(u32::from(systemd_journal_gid)),
        )?;*/

        let mut perms = fs::metadata(&node_path).await?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&node_path, perms).await?;

        Ok(node_path)
    }

    pub async fn ensure_capacity(&self) -> Result<bool> {
        let count = self.db.count_bees().await?;
        if count >= 99 {
            return Ok(false);
        }
        return Ok(true);
    }

    pub async fn get_new_bee_id(&self) -> Result<u8> {
        let bees = self.get_bees().await?;
        let mut available_ids = (1..99).collect::<Vec<u8>>();

        for bee in bees {
            available_ids.retain(|id| *id != bee.id);
        }

        available_ids
            .first()
            .ok_or(anyhow::anyhow!("Unable to get new bee id"))
            .map(|v| v.clone())
    }

    pub async fn save_bee(&self, bee_data: BeeData) -> Result<BeeData> {
        if !self.ensure_capacity().await? {
            return Err(anyhow!("Max capacity reached"));
        }

        self.db.add_bee(bee_data.clone()).await?;
        Ok(bee_data)
    }

    pub async fn get_bee(&self, bee_id: u8) -> Result<Option<BeeData>> {
        self.db.get_bee(bee_id).await
    }

    pub async fn get_bees(&self) -> Result<Vec<BeeData>> {
        self.db.get_bees().await
    }

    pub async fn count_bees(&self) -> Result<u64> {
        self.db.count_bees().await
    }

    pub async fn delete_bee(&self, bee_id: u8) -> Result<()> {
        let node_path = self.get_node_path(bee_id)?;
        fs::remove_dir_all(node_path).await?;
        self.db.delete_bee(bee_id).await?;
        Ok(())
    }

    pub fn data_to_info(&self, data: &BeeData) -> Result<BeeInfo> {
        let api_port = &self.get_api_port(data.id)?;
        let p2p_port = &self.get_p2p_port(data.id)?;
        Ok(BeeInfo::new(
            data,
            &self.config.bee.image,
            &self.config.bee.password_path,
            api_port,
            p2p_port,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::Storage;
    use crate::services::db_service::MockDbService;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn should_format_id() {
        assert_eq!(BeeService::format_id(5), "05");
        assert_eq!(BeeService::format_id(40), "40");
        assert_eq!(BeeService::format_id(99), "99");
    }

    #[tokio::test]
    async fn should_return_name_from_id() {
        assert_eq!(BeeService::get_node_name(5), "node_05");
    }

    #[tokio::test]
    async fn should_return_port_from_id_and_base_port() {
        let id = 5;
        let base_port = "17xx";
        let expected_port = "1705";

        let port = BeeService::get_port(id, base_port).unwrap();

        assert_eq!(port, expected_port);
    }

    #[tokio::test]
    async fn should_fail_to_return_port_from_invalid_base_port() {
        assert!(BeeService::get_port(5, "1705").is_err());
        assert!(BeeService::get_port(5, "test").is_err());
        assert!(BeeService::get_port(5, "1x70").is_err());
        assert!(BeeService::get_port(5, "1xx0").is_err());
        assert!(BeeService::get_port(5, "15340xx").is_err());
    }

    #[tokio::test]
    async fn should_return_neighborhood_from_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/network/neighborhoods/suggestion"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "neighborhood": "11111111111"
            })))
            .mount(&mock_server)
            .await;

        let url = format!("{}/v1/network/neighborhoods/suggestion", mock_server.uri());
        env::set_var("NEIGHBORHOOD_API_URL", url);

        let result = BeeService::get_neighborhood().await.unwrap();

        assert_eq!(result, "11111111111");
    }

    #[tokio::test]
    async fn should_throw_error_when_neighborhood_field_is_missing() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&mock_server)
            .await;

        let url = format!("{}/v1/network/neighborhoods/suggestion", mock_server.uri());
        env::set_var("NEIGHBORHOOD_API_URL", url);

        let result = BeeService::get_neighborhood().await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Missing 'neighborhood' field"
        );
    }

    #[tokio::test]
    async fn should_throw_error_when_http_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let url = format!("{}/v1/network/neighborhoods/suggestion", mock_server.uri());
        env::set_var("NEIGHBORHOOD_API_URL", url);

        let result = BeeService::get_neighborhood().await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn should_return_api_port_from_config() {
        let config = Config {
            network: crate::models::config::Network {
                api_port: "17xx".to_string(),
                p2p_port: "18xx".to_string(),
                ..Default::default()
            },
            ..Config::default()
        };
        let service = BeeService::new(config, Box::new(MockDbService::default()));

        let api_port = service.get_api_port(5).unwrap();
        let p2p_port = service.get_p2p_port(5).unwrap();

        assert_eq!(api_port, "1705");
        assert_eq!(p2p_port, "1805");
    }

    #[tokio::test]
    async fn should_fail_api_port_with_invalid_base() {
        let config = Config {
            network: crate::models::config::Network {
                api_port: "1705".to_string(),
                ..Default::default()
            },
            ..Config::default()
        };
        let service = BeeService::new(config, Box::new(MockDbService::default()));

        assert!(service.get_api_port(5).is_err());
    }

    #[tokio::test]
    async fn should_fail_p2p_port_with_invalid_base() {
        let config = Config {
            network: crate::models::config::Network {
                p2p_port: "test".to_string(),
                ..Default::default()
            },
            ..Config::default()
        };
        let service = BeeService::new(config, Box::new(MockDbService::default()));

        assert!(service.get_p2p_port(5).is_err());
    }

    #[tokio::test]
    async fn should_calculate_directory_id_correctly() {
        let mut config = Config::default();

        config.storage.parent_dir_capacity = 4;
        let mut bee_service = BeeService::new(config.clone(), Box::new(MockDbService::default()));
        assert_eq!(bee_service.get_dir_id(1), 1);
        assert_eq!(bee_service.get_dir_id(4), 1);
        assert_eq!(bee_service.get_dir_id(5), 2);
        assert_eq!(bee_service.get_dir_id(8), 2);
        assert_eq!(bee_service.get_dir_id(9), 3);
        assert_eq!(bee_service.get_dir_id(99), 25);

        config.storage.parent_dir_capacity = 3;
        bee_service = BeeService::new(config.clone(), Box::new(MockDbService::default()));
        assert_eq!(bee_service.get_dir_id(3), 1);
        assert_eq!(bee_service.get_dir_id(4), 2);
        assert_eq!(bee_service.get_dir_id(6), 2);
        assert_eq!(bee_service.get_dir_id(7), 3);

        config.storage.parent_dir_capacity = 5;
        bee_service = BeeService::new(config.clone(), Box::new(MockDbService::default()));
        assert_eq!(bee_service.get_dir_id(5), 1);
        assert_eq!(bee_service.get_dir_id(6), 2);
    }

    #[tokio::test]
    async fn should_generate_directory_name_correctly() {
        let mut config = Config {
            storage: Storage {
                parent_dir_format: String::from("swarm_data_xx"),
                parent_dir_capacity: 4,
                ..Storage::default()
            },
            ..Config::default()
        };

        let mut bee_service = BeeService::new(config.clone(), Box::new(MockDbService::default()));

        assert_eq!(bee_service.get_parent_dir_name(1).unwrap(), "swarm_data_01");

        assert_eq!(bee_service.get_parent_dir_name(5).unwrap(), "swarm_data_02");

        assert_eq!(bee_service.get_parent_dir_name(9).unwrap(), "swarm_data_03");

        config.storage.parent_dir_capacity = 3;
        bee_service = BeeService::new(config.clone(), Box::new(MockDbService::default()));
        assert_eq!(bee_service.get_parent_dir_name(4).unwrap(), "swarm_data_02");
    }

    #[tokio::test]
    async fn should_return_error_for_invalid_volume_name_format() {
        let config = Config {
            storage: Storage {
                parent_dir_format: String::from("swarm_data_x"),
                parent_dir_capacity: 4,
                ..Storage::default()
            },
            ..Config::default()
        };
        let bee_service = BeeService::new(config, Box::new(MockDbService::default()));

        let result = bee_service.get_parent_dir_name(1);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Invalid parent name format 'swarm_data_x'"
        );
    }

    #[tokio::test]
    async fn should_generate_correct_node_path() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root_path = temp_dir.path().to_path_buf();
        let config = Config {
            storage: Storage {
                root_path: root_path.clone(),
                parent_dir_format: "swarm_data_xx".to_string(),
                parent_dir_capacity: 4,
                ..Storage::default()
            },
            ..Config::default()
        };
        let bee_service = BeeService::new(config, Box::new(MockDbService::default()));

        let path = bee_service.get_node_path(5).unwrap();

        assert_eq!(path, root_path.join("swarm_data_02").join("node_05"));
    }

    #[tokio::test]
    async fn should_fail_node_path_with_invalid_parent_format() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root_path = temp_dir.path().to_path_buf();
        let config = Config {
            storage: Storage {
                root_path: root_path.clone(),
                parent_dir_format: "swarm_data_x".to_string(),
                parent_dir_capacity: 4,
                ..Storage::default()
            },
            ..Config::default()
        };
        let bee_service = BeeService::new(config, Box::new(MockDbService::default()));

        let result = bee_service.get_node_path(1);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Invalid parent name format 'swarm_data_x'"
        );
    }

    #[tokio::test]
    async fn should_get_correct_path_for_first_id() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root_path = temp_dir.path().to_path_buf();
        let config = Config {
            storage: Storage {
                root_path: root_path.clone(),
                parent_dir_format: "data_xx".to_string(),
                parent_dir_capacity: 3,
                ..Storage::default()
            },
            ..Config::default()
        };
        let bee_service = BeeService::new(config, Box::new(MockDbService::default()));

        let path = bee_service.get_node_path(1).unwrap();

        assert_eq!(path, root_path.join("data_01").join("node_01"));
    }

    #[tokio::test]
    async fn should_get_correct_path_for_max_id() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root_path = temp_dir.path().to_path_buf();
        let config = Config {
            storage: Storage {
                root_path: root_path.clone(),
                parent_dir_format: "storage_xx".to_string(),
                parent_dir_capacity: 4,
                ..Storage::default()
            },
            ..Config::default()
        };
        let bee_service = BeeService::new(config, Box::new(MockDbService::default()));

        let path = bee_service.get_node_path(99).unwrap();

        assert_eq!(path, root_path.join("storage_25").join("node_99"));
    }

    #[tokio::test]
    async fn should_create_node_dir_successfully() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root_path: PathBuf = temp_dir.path().into();
        let config = Config {
            storage: Storage {
                root_path: root_path.clone(),
                parent_dir_format: String::from("swarm_data_xx"),
                parent_dir_capacity: 4,
                ..Storage::default()
            },
            ..Config::default()
        };
        let bee_service = BeeService::new(config, Box::new(MockDbService::default()));

        let result = bee_service.create_node_dir(1).await;

        assert!(result.is_ok());
        let node_path = result.unwrap();

        assert!(node_path.exists());
        assert_eq!(node_path, root_path.join("swarm_data_01").join("node_01"));

        let metadata = tokio::fs::metadata(&node_path).await.unwrap();
        assert_eq!(metadata.permissions().mode() & 0o777, 0o755);
    }

    #[tokio::test]
    async fn should_fail_to_create_node_dir_if_dir_already_exists() {
        let root_path: PathBuf = tempfile::tempdir().unwrap().path().into();
        let config = Config {
            storage: Storage {
                root_path: root_path.clone(),
                parent_dir_format: String::from("swarm_data_xx"),
                parent_dir_capacity: 4,
                ..Storage::default()
            },
            ..Config::default()
        };
        let bee_service = BeeService::new(config, Box::new(MockDbService::default()));
        let existing_path = root_path.join("swarm_data_01").join("node_01");
        tokio::fs::create_dir_all(&existing_path).await.unwrap();

        let result = bee_service.create_node_dir(1).await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("Directory '{}' already exists", existing_path.display())
        );
    }

    #[tokio::test]
    async fn should_not_fail_to_create_node_dir_if_parent_dir_already_exists() {
        let root_path: PathBuf = tempfile::tempdir().unwrap().path().into();
        let config = Config {
            storage: Storage {
                root_path: root_path.clone(),
                parent_dir_format: String::from("swarm_data_xx"),
                parent_dir_capacity: 4,
                ..Storage::default()
            },
            ..Config::default()
        };
        let bee_service = BeeService::new(config, Box::new(MockDbService::default()));
        let existing_path = root_path.join("swarm_data_01").join("node_02");
        tokio::fs::create_dir_all(&existing_path).await.unwrap();

        let result = bee_service.create_node_dir(1).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn should_fail_to_create_node_dir_if_invalid_dir_format() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root_path: PathBuf = temp_dir.path().into();
        let config = Config {
            storage: Storage {
                root_path,
                parent_dir_format: String::from("swarm_data_x"),
                parent_dir_capacity: 4,
                ..Storage::default()
            },
            ..Config::default()
        };
        let bee_service = BeeService::new(config, Box::new(MockDbService::default()));

        let result = bee_service.create_node_dir(1).await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Invalid parent name format 'swarm_data_x'"
        );
    }

    #[tokio::test]
    async fn ensure_capacity_returns_true_under_99() {
        let mock_db = MockDbService::default();
        for id in 1..99 {
            mock_db
                .add_bee(BeeData {
                    id,
                    ..Default::default()
                })
                .await
                .unwrap();
        }
        let service = BeeService::new(Config::default(), Box::new(mock_db));

        let capacity = service.ensure_capacity().await.unwrap();

        assert!(capacity, "ensure_capacity should return true when under 99");
    }

    #[tokio::test]
    async fn ensure_capacity_returns_false_at_99() {
        let mock_db = MockDbService::default();
        for id in 1..=99 {
            mock_db
                .add_bee(BeeData {
                    id,
                    ..Default::default()
                })
                .await
                .unwrap();
        }
        let service = BeeService::new(Config::default(), Box::new(mock_db));

        let capacity = service.ensure_capacity().await.unwrap();

        assert!(!capacity, "ensure_capacity should return false at 99");
    }

    #[tokio::test]
    async fn ensure_capacity_returns_true_when_empty() {
        let mock_db = MockDbService::default();
        let service = BeeService::new(Config::default(), Box::new(mock_db));

        let capacity = service.ensure_capacity().await.unwrap();

        assert!(
            capacity,
            "ensure_capacity should return true when no bees exist"
        );
    }

    #[tokio::test]
    async fn should_get_next_bee_id() {
        let db = MockDbService::default();
        let bee_service = BeeService::new(Config::default(), Box::new(db.clone()));

        db.add_bee(BeeData {
            id: 1,
            ..Default::default()
        })
        .await
        .unwrap();

        db.add_bee(BeeData {
            id: 2,
            ..Default::default()
        })
        .await
        .unwrap();

        let new_bee_id = bee_service.get_new_bee_id().await.unwrap();

        assert_eq!(new_bee_id, 3);
    }

    #[tokio::test]
    async fn should_pick_first_available_id() {
        let db = MockDbService::default();
        let bee_service = BeeService::new(Config::default(), Box::new(db.clone()));

        db.add_bee(BeeData {
            id: 1,
            ..Default::default()
        })
        .await
        .unwrap();
        db.add_bee(BeeData {
            id: 3,
            ..Default::default()
        })
        .await
        .unwrap();

        let new_bee_id = bee_service.get_new_bee_id().await.unwrap();

        assert_eq!(new_bee_id, 2);
    }

    #[tokio::test]
    async fn should_fail_to_get_new_bee_id_when_all_ids_are_taken() {
        let db = MockDbService::default();
        let bee_service = BeeService::new(Config::default(), Box::new(db.clone()));

        for id in 1..=99 {
            db.add_bee(BeeData {
                id,
                ..Default::default()
            })
            .await
            .unwrap();
        }

        let result = bee_service.get_new_bee_id().await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Unable to get new bee id".to_string()
        );
    }

    #[tokio::test]
    async fn should_save_first_bee() {
        let db = MockDbService::default();
        let bee_service = BeeService::new(Config::default(), Box::new(db.clone()));
        let bee_data = BeeData {
            id: 1,
            ..Default::default()
        };

        let new_bee = bee_service.save_bee(bee_data).await.unwrap();

        assert_eq!(new_bee.id, 1);
        assert_eq!(db.count_bees().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn should_fail_saving_when_max_capacity_reached() {
        let db = MockDbService::default();
        let bee_service = BeeService::new(Config::default(), Box::new(db.clone()));

        for id in 1..=99 {
            db.add_bee(BeeData {
                id,
                ..Default::default()
            })
            .await
            .unwrap();
        }

        let result = bee_service.save_bee(BeeData::default()).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn should_delete_bee_with_nested_node_directory() {
        let db = MockDbService::default();
        let root_path = tempfile::tempdir().unwrap().path().to_path_buf();
        let config = Config {
            storage: Storage {
                root_path: root_path.clone(),
                parent_dir_format: "swarm_data_xx".to_string(),
                parent_dir_capacity: 4,
                ..Storage::default()
            },
            ..Config::default()
        };
        let bee_service = BeeService::new(config, Box::new(db.clone()));
        let bee_id = 1;
        db.add_bee(BeeData {
            id: bee_id,
            ..BeeData::default()
        })
        .await
        .unwrap();
        let node_path = bee_service.get_node_path(bee_id).unwrap();
        tokio::fs::create_dir_all(&node_path).await.unwrap();
        let nested_file_path = node_path.join("nested_file.txt");
        tokio::fs::write(&nested_file_path, "test content")
            .await
            .unwrap();
        assert!(nested_file_path.exists());

        bee_service.delete_bee(bee_id).await.unwrap();

        assert!(bee_service.get_bee(bee_id).await.unwrap().is_none());
        assert!(!node_path.exists());
        assert!(!nested_file_path.exists());
    }

    #[tokio::test]
    async fn should_convert_bee_data_to_info() {
        let config = Config {
            network: crate::models::config::Network {
                api_port: "17xx".to_string(),
                p2p_port: "18xx".to_string(),
                ..Default::default()
            },
            bee: crate::models::config::Bee {
                image: "bee-image:latest".to_string(),
                password_path: "/etc/bee/password".to_string(),
                ..Default::default()
            },
            ..Config::default()
        };
        let db = MockDbService::default();
        let service = BeeService::new(config, Box::new(db));

        let bee_data = BeeData {
            id: 5,
            ..Default::default()
        };

        let bee_info = service.data_to_info(&bee_data).unwrap();

        assert_eq!(bee_info.api_port, "1705");
        assert_eq!(bee_info.p2p_port, "1805");
        assert_eq!(bee_info.image, "bee-image:latest");
        assert_eq!(bee_info.password_path, "/etc/bee/password");
    }
}

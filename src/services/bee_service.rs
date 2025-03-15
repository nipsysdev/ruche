use crate::config::Config;
use crate::constants::NEIGHBORHOOD_API_URL;
use crate::models::BeeData;
use crate::services::db_service::BeeDatabase;
use crate::utils::regex::{PORT_REGEX, VOLUME_NAME_REGEX};
use anyhow::{anyhow, Error, Result};
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

    pub fn get_name(id: u8) -> String {
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
            .to_string())
    }

    pub fn get_dir_id(&self, bee_id: u8) -> u8 {
        ((bee_id - 1) / self.config.storage.node_qty_per_volume) + 1
    }

    pub fn get_dir_name(&self, bee_id: u8) -> Result<String> {
        let dir_name_format = &self.config.storage.volume_name;

        let re = Regex::new(VOLUME_NAME_REGEX)?;
        if !re.is_match(dir_name_format) {
            return Err(anyhow!("Invalid volume name format '{}'", dir_name_format));
        }

        Ok(dir_name_format.replace("xx", &Self::format_id(self.get_dir_id(bee_id))))
    }

    pub async fn create_node_dir(&self, id: u8) -> Result<PathBuf> {
        let base_path = &self.config.storage.volumes_parent;
        let dir_name = self.get_dir_name(id)?;
        let dir_path = Path::new(base_path).join(dir_name);

        if dir_path.exists() {
            return Err(anyhow!("Directory '{}' already exists", dir_path.display()));
        }

        fs::create_dir_all(&dir_path).await?;

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

        let mut perms = fs::metadata(&dir_path).await?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dir_path, perms).await?;

        Ok(dir_path)
    }

    pub async fn save_bee(&self) -> Result<BeeData> {
        let count = self.db.count_bees().await?;
        if count >= 99 {
            return Err(anyhow!("Max capacity reached"));
        }
        let bees = self.get_bees().await?;
        let mut available_ids = (1..99).collect::<Vec<u8>>();

        for bee in bees {
            available_ids.retain(|id| *id != bee.id);
        }

        let new_id = available_ids
            .first()
            .ok_or(anyhow::anyhow!("Unable to get new bee id"))?;

        let bee = BeeData {
            id: *new_id,
            neighborhood: String::new(),
            reserve_doubling: false,
        };

        self.db.add_bee(bee.clone()).await?;
        Ok(bee)
    }

    pub async fn get_bees(&self) -> Result<Vec<BeeData>> {
        let cursor = self.db.get_bees().await?;
        let mut bees = Vec::new();
        for result in cursor {
            let bee = result.map_err(Error::from)?;
            bees.push(bee);
        }
        Ok(bees)
    }

    pub async fn delete_bee(&self, bee_id: u8) -> Result<()> {
        self.db.delete_bee(bee_id).await?;
        Ok(())
    }
}

/*#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(BeeService::get_name(5), "node_05");
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
    async fn should_calculate_directory_id_correctly() {
        assert_eq!(BeeService::get_dir_id(1, 4), 1);
        assert_eq!(BeeService::get_dir_id(4, 4), 1);
        assert_eq!(BeeService::get_dir_id(5, 4), 2);
        assert_eq!(BeeService::get_dir_id(8, 4), 2);
        assert_eq!(BeeService::get_dir_id(9, 4), 3);
        assert_eq!(BeeService::get_dir_id(99, 4), 25);

        assert_eq!(BeeService::get_dir_id(3, 3), 1);
        assert_eq!(BeeService::get_dir_id(4, 3), 2);
        assert_eq!(BeeService::get_dir_id(6, 3), 2);
        assert_eq!(BeeService::get_dir_id(7, 3), 3);

        assert_eq!(BeeService::get_dir_id(5, 5), 1);
        assert_eq!(BeeService::get_dir_id(6, 5), 2);
    }

    #[tokio::test]
    async fn should_generate_directory_name_correctly() {
        let dir_name_format = "node_xx";
        let dir_capacity = 4;

        assert_eq!(
            BeeService::get_dir_name(1, dir_name_format, dir_capacity).unwrap(),
            "node_01"
        );

        assert_eq!(
            BeeService::get_dir_name(5, dir_name_format, dir_capacity).unwrap(),
            "node_02"
        );

        assert_eq!(
            BeeService::get_dir_name(9, dir_name_format, dir_capacity).unwrap(),
            "node_03"
        );

        let dir_capacity_3 = 3;
        assert_eq!(
            BeeService::get_dir_name(4, dir_name_format, dir_capacity_3).unwrap(),
            "node_02"
        );
    }

    #[tokio::test]
    async fn should_return_error_for_invalid_volume_name_format() {
        let invalid_format = "node_x";
        let dir_capacity = 4;

        let result = BeeService::get_dir_name(1, invalid_format, dir_capacity);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Invalid volume name format 'node_x'"
        );
    }

    #[tokio::test]
    async fn should_create_node_dir_successfully() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_path = temp_dir.path().to_str().unwrap();
        let dir_name_format = "node_xx";
        let dir_capacity = 4;
        let bee_id = 1;

        let result =
            BeeService::create_node_dir(bee_id, base_path, dir_name_format, dir_capacity).await;

        assert!(result.is_ok());
        let dir_path = result.unwrap();

        assert!(dir_path.exists());
        assert_eq!(dir_path.file_name().unwrap().to_str().unwrap(), "node_01");

        let metadata = tokio::fs::metadata(&dir_path).await.unwrap();
        assert_eq!(metadata.permissions().mode() & 0o777, 0o755);
    }

    #[tokio::test]
    async fn should_fail_to_create_node_dir_if_dir_already_exists() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_path = temp_dir.path().to_str().unwrap();
        let existing_dir_name = "node_01";
        let existing_path = temp_dir.path().join(existing_dir_name);

        tokio::fs::create_dir_all(&existing_path).await.unwrap();

        let result = BeeService::create_node_dir(1, base_path, "node_xx", 4).await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            format!("Directory '{}' already exists", existing_path.display())
        );
    }

    #[tokio::test]
    async fn should_fail_to_create_node_dir_if_invalid_dir_format() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_path = temp_dir.path().to_str().unwrap();

        let result = BeeService::create_node_dir(1, base_path, "node_x", 4).await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Invalid volume name format 'node_x'"
        );
    }

    #[tokio::test]
    async fn should_save_first_bee() {
        let mock = MockDbService::default();

        let new_bee = BeeService::save_bee(&mock).await.unwrap();

        assert_eq!(new_bee.id, 1);
        assert_eq!(mock.count_bees().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn should_add_additional_bee() {
        let mock = MockDbService::default();

        mock.add_bee(BeeData {
            id: 1,
            neighborhood: String::new(),
            reserve_doubling: false,
        })
        .await
        .unwrap();

        mock.add_bee(BeeData {
            id: 2,
            neighborhood: String::new(),
            reserve_doubling: false,
        })
        .await
        .unwrap();

        let new_bee = BeeService::save_bee(&mock).await.unwrap();

        assert_eq!(new_bee.id, 3);
        assert_eq!(mock.count_bees().await.unwrap(), 3);
    }

    #[tokio::test]
    async fn should_fail_saving_when_max_capacity_reached() {
        let mock = MockDbService::default();

        for id in 1..=99 {
            mock.add_bee(BeeData {
                id,
                neighborhood: String::new(),
                reserve_doubling: false,
            })
            .await
            .unwrap();
        }

        let result = BeeService::save_bee(&mock).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn should_pick_first_available_id() {
        let mock = MockDbService::default();

        mock.add_bee(BeeData {
            id: 1,
            neighborhood: String::new(),
            reserve_doubling: false,
        })
        .await
        .unwrap();
        mock.add_bee(BeeData {
            id: 3,
            neighborhood: String::new(),
            reserve_doubling: false,
        })
        .await
        .unwrap();

        let new_bee = BeeService::save_bee(&mock).await.unwrap();

        assert_eq!(new_bee.id, 2);
        assert_eq!(mock.count_bees().await.unwrap(), 3);
    }
}*/

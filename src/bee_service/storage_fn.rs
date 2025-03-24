use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use regex::Regex;
use tokio::fs;

use crate::{models::config::Config, utils::regex::VOLUME_NAME_REGEX};

use super::bee_fn::{format_id, get_node_name};

pub fn get_dir_id(config: &Config, bee_id: u8) -> u8 {
    ((bee_id - 1) / config.storage.parent_dir_capacity) + 1
}

pub fn get_parent_dir_name(config: &Config, bee_id: u8) -> Result<String> {
    let dir_name_format = &config.storage.parent_dir_format;

    let re = Regex::new(VOLUME_NAME_REGEX)?;
    if !re.is_match(dir_name_format) {
        return Err(anyhow!("Invalid parent name format '{}'", dir_name_format));
    }

    Ok(dir_name_format.replace("xx", &format_id(get_dir_id(config, bee_id))))
}

pub fn get_node_path(config: &Config, bee_id: u8) -> Result<PathBuf> {
    let root_path = &config.storage.root_path;
    let parent_name = get_parent_dir_name(config, bee_id)?;
    let parent_path = Path::new(root_path).join(parent_name);
    Ok(parent_path.join(get_node_name(bee_id)))
}

pub async fn create_node_dir(config: &Config, bee_id: u8) -> Result<PathBuf> {
    let node_path = get_node_path(config, bee_id)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::Storage;

    #[tokio::test]
    async fn should_calculate_directory_id_correctly() {
        let mut config = Config::default();

        config.storage.parent_dir_capacity = 4;
        assert_eq!(get_dir_id(&config, 1), 1);
        assert_eq!(get_dir_id(&config, 4), 1);
        assert_eq!(get_dir_id(&config, 5), 2);
        assert_eq!(get_dir_id(&config, 8), 2);
        assert_eq!(get_dir_id(&config, 9), 3);
        assert_eq!(get_dir_id(&config, 99), 25);

        config.storage.parent_dir_capacity = 3;
        assert_eq!(get_dir_id(&config, 3), 1);
        assert_eq!(get_dir_id(&config, 4), 2);
        assert_eq!(get_dir_id(&config, 6), 2);
        assert_eq!(get_dir_id(&config, 7), 3);

        config.storage.parent_dir_capacity = 5;
        assert_eq!(get_dir_id(&config, 5), 1);
        assert_eq!(get_dir_id(&config, 6), 2);
    }

    #[tokio::test]
    async fn should_generate_directory_name_correctly() {
        let mut config = Config {
            storage: Storage {
                parent_dir_format: String::from("swarm_data_xx"),
                parent_dir_capacity: 4,
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(get_parent_dir_name(&config, 1).unwrap(), "swarm_data_01");

        assert_eq!(get_parent_dir_name(&config, 5).unwrap(), "swarm_data_02");

        assert_eq!(get_parent_dir_name(&config, 9).unwrap(), "swarm_data_03");

        config.storage.parent_dir_capacity = 3;
        assert_eq!(get_parent_dir_name(&config, 4).unwrap(), "swarm_data_02");
    }

    #[tokio::test]
    async fn should_return_error_for_invalid_volume_name_format() {
        let config = Config {
            storage: Storage {
                parent_dir_format: String::from("swarm_data_x"),
                parent_dir_capacity: 4,
                ..Default::default()
            },
            ..Default::default()
        };

        let result = get_parent_dir_name(&config, 1);
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
                ..Default::default()
            },
            ..Default::default()
        };

        let path = get_node_path(&config, 5).unwrap();

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
                ..Default::default()
            },
            ..Default::default()
        };

        let result = get_node_path(&config, 1);

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
                ..Default::default()
            },
            ..Default::default()
        };

        let path = get_node_path(&config, 1).unwrap();

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
                ..Default::default()
            },
            ..Default::default()
        };

        let path = get_node_path(&config, 99).unwrap();

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
                ..Default::default()
            },
            ..Default::default()
        };

        let result = create_node_dir(&config, 1).await;

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
                ..Default::default()
            },
            ..Default::default()
        };
        let existing_path = root_path.join("swarm_data_01").join("node_01");
        tokio::fs::create_dir_all(&existing_path).await.unwrap();

        let result = create_node_dir(&config, 1).await;

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
                ..Default::default()
            },
            ..Default::default()
        };
        let existing_path = root_path.join("swarm_data_01").join("node_02");
        tokio::fs::create_dir_all(&existing_path).await.unwrap();

        let result = create_node_dir(&config, 1).await;

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
                ..Default::default()
            },
            ..Default::default()
        };

        let result = create_node_dir(&config, 1).await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Invalid parent name format 'swarm_data_x'"
        );
    }
}

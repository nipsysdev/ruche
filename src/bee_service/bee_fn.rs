use std::path::PathBuf;

use crate::{
    core::{database::BeeDatabase, docker::BeeDocker},
    models::{
        bee::{BeeData, BeeInfo},
        config::Config,
    },
};
use anyhow::{anyhow, Result};
use tokio::fs;

use super::{
    network_fn::{get_api_port, get_p2p_port},
    storage_fn::get_node_path,
};

pub fn format_id(id: u8) -> String {
    format!("{:02}", id)
}

pub fn get_node_name(id: u8) -> String {
    format!("node_{}", format_id(id))
}

pub async fn ensure_capacity(db: Box<dyn BeeDatabase>) -> Result<bool> {
    let count = db.count_bees().await?;
    if count >= 99 {
        return Ok(false);
    }
    return Ok(true);
}

pub async fn get_new_bee_id(db: Box<dyn BeeDatabase>) -> Result<u8> {
    let bees = get_bees(db).await?;
    let mut available_ids = (1..99).collect::<Vec<u8>>();

    for bee in bees {
        available_ids.retain(|id| *id != bee.id);
    }

    available_ids
        .first()
        .ok_or(anyhow::anyhow!("Unable to get new bee id"))
        .map(|v| v.clone())
}

pub fn new_bee_data(config: &Config, id: u8, neighborhood: &str, data_dir: &PathBuf) -> BeeData {
    BeeData {
        id,
        neighborhood: neighborhood.to_owned(),
        data_dir: data_dir.to_owned(),
        full_node: config.bee.full_node,
        swap_enable: config.bee.swap_enable,
        reserve_doubling: config.bee.reserve_doubling,
    }
}

pub async fn save_bee(db: Box<dyn BeeDatabase>, bee_data: &BeeData) -> Result<()> {
    if !ensure_capacity(db.clone()).await? {
        return Err(anyhow!("Max capacity reached"));
    }

    db.add_bee(bee_data.to_owned()).await?;
    Ok(())
}

pub fn data_to_info(config: &Config, data: &BeeData) -> Result<BeeInfo> {
    let api_port = &get_api_port(config, data.id)?;
    let p2p_port = &get_p2p_port(config, data.id)?;
    Ok(BeeInfo::new(data, &config.bee.image, api_port, p2p_port))
}

pub async fn get_bee(db: Box<dyn BeeDatabase>, bee_id: u8) -> Result<Option<BeeData>> {
    db.get_bee(bee_id).await
}

pub async fn get_bees(db: Box<dyn BeeDatabase>) -> Result<Vec<BeeData>> {
    db.get_bees().await
}

pub async fn count_bees(db: Box<dyn BeeDatabase>) -> Result<u64> {
    db.count_bees().await
}

pub async fn delete_bee(config: &Config, db: Box<dyn BeeDatabase>, bee_id: u8) -> Result<()> {
    let node_path = get_node_path(config, bee_id)?;
    fs::remove_dir_all(node_path).await?;
    db.delete_bee(bee_id).await?;
    Ok(())
}

pub async fn create_bee_container(
    config: &Config,
    docker: Box<dyn BeeDocker>,
    bee: &BeeInfo,
) -> Result<()> {
    docker.new_bee_container(bee, config).await
}

pub async fn start_bee_container(docker: Box<dyn BeeDocker>, name: &str) -> Result<()> {
    docker.start_bee_container(name).await
}

pub async fn stop_bee_container(docker: Box<dyn BeeDocker>, name: &str) -> Result<()> {
    docker.stop_bee_container(name).await
}

pub async fn remove_bee_container(docker: Box<dyn BeeDocker>, name: &str) -> Result<()> {
    docker.remove_bee_container(name).await
}

pub async fn get_bee_container_logs(docker: Box<dyn BeeDocker>, name: &str) -> Result<Vec<String>> {
    docker.get_bee_container_logs(name).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::database::MockDbService, models::config::Storage};

    #[tokio::test]
    async fn should_format_id() {
        assert_eq!(format_id(5), "05");
        assert_eq!(format_id(40), "40");
        assert_eq!(format_id(99), "99");
    }

    #[tokio::test]
    async fn should_return_name_from_id() {
        assert_eq!(get_node_name(5), "node_05");
    }

    #[tokio::test]
    async fn ensure_capacity_returns_true_under_99() {
        let db = Box::new(MockDbService::default());
        for id in 1..99 {
            db.add_bee(BeeData {
                id,
                ..Default::default()
            })
            .await
            .unwrap();
        }

        let capacity = ensure_capacity(db).await.unwrap();

        assert!(capacity, "ensure_capacity should return true when under 99");
    }

    #[tokio::test]
    async fn ensure_capacity_returns_false_at_99() {
        let db = Box::new(MockDbService::default());
        for id in 1..=99 {
            db.add_bee(BeeData {
                id,
                ..Default::default()
            })
            .await
            .unwrap();
        }

        let capacity = ensure_capacity(db).await.unwrap();

        assert!(!capacity, "ensure_capacity should return false at 99");
    }

    #[tokio::test]
    async fn ensure_capacity_returns_true_when_empty() {
        let db = Box::new(MockDbService::default());

        let capacity = ensure_capacity(db).await.unwrap();

        assert!(
            capacity,
            "ensure_capacity should return true when no bees exist"
        );
    }

    #[tokio::test]
    async fn should_get_next_bee_id() {
        let db = Box::new(MockDbService::default());
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

        let new_bee_id = get_new_bee_id(db).await.unwrap();

        assert_eq!(new_bee_id, 3);
    }

    #[tokio::test]
    async fn should_pick_first_available_id() {
        let db = Box::new(MockDbService::default());
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

        let new_bee_id = get_new_bee_id(db).await.unwrap();

        assert_eq!(new_bee_id, 2);
    }

    #[tokio::test]
    async fn should_fail_to_get_new_bee_id_when_all_ids_are_taken() {
        let db = Box::new(MockDbService::default());
        for id in 1..=99 {
            db.add_bee(BeeData {
                id,
                ..Default::default()
            })
            .await
            .unwrap();
        }

        let result = get_new_bee_id(db).await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Unable to get new bee id".to_string()
        );
    }

    #[tokio::test]
    async fn should_create_new_bee_data_correctly() {
        let config = Config {
            bee: crate::models::config::Bee {
                full_node: false,
                swap_enable: true,
                reserve_doubling: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let id = 5;
        let neighborhood = "test_neighborhood";
        let data_dir = PathBuf::from("/tmp/test_dir");

        let bee_data = new_bee_data(&config, id, neighborhood, &data_dir);

        assert_eq!(bee_data.id, id);
        assert_eq!(bee_data.neighborhood, neighborhood);
        assert_eq!(bee_data.data_dir, data_dir);
        assert_eq!(bee_data.full_node, config.bee.full_node);
        assert_eq!(bee_data.swap_enable, config.bee.swap_enable);
        assert_eq!(bee_data.reserve_doubling, config.bee.reserve_doubling);
    }

    #[tokio::test]
    async fn should_handle_empty_neighborhood_in_new_bee_data() {
        let config = Config {
            bee: crate::models::config::Bee {
                full_node: true,
                swap_enable: false,
                reserve_doubling: false,
                ..Default::default()
            },
            ..Default::default()
        };

        let id = 10;
        let neighborhood = "";
        let data_dir = PathBuf::from("/another/path");

        let bee_data = new_bee_data(&config, id, neighborhood, &data_dir);

        assert_eq!(bee_data.neighborhood, "");
        assert_eq!(bee_data.full_node, config.bee.full_node);
        assert_eq!(bee_data.swap_enable, config.bee.swap_enable);
        assert_eq!(bee_data.reserve_doubling, config.bee.reserve_doubling);
    }

    #[tokio::test]
    async fn should_save_first_bee() {
        let db = Box::new(MockDbService::default());
        let bee_data = BeeData {
            id: 1,
            ..Default::default()
        };

        save_bee(db.clone(), &bee_data).await.unwrap();

        assert_eq!(db.count_bees().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn should_fail_saving_when_max_capacity_reached() {
        let db = Box::new(MockDbService::default());
        for id in 1..=99 {
            db.add_bee(BeeData {
                id,
                ..Default::default()
            })
            .await
            .unwrap();
        }

        let result = save_bee(db, &BeeData::default()).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn should_delete_bee_with_nested_node_directory() {
        let db = Box::new(MockDbService::default());
        let root_path = tempfile::tempdir().unwrap().path().to_path_buf();
        let config = Config {
            storage: Storage {
                root_path: root_path.clone(),
                parent_dir_format: "swarm_data_xx".to_string(),
                parent_dir_capacity: 4,
                ..Default::default()
            },
            ..Default::default()
        };
        let bee_id = 1;
        db.add_bee(BeeData {
            id: bee_id,
            ..Default::default()
        })
        .await
        .unwrap();
        let node_path = get_node_path(&config, bee_id).unwrap();
        tokio::fs::create_dir_all(&node_path).await.unwrap();
        let nested_file_path = node_path.join("nested_file.txt");
        tokio::fs::write(&nested_file_path, "test content")
            .await
            .unwrap();
        assert!(nested_file_path.exists());

        delete_bee(&config, db.clone(), bee_id).await.unwrap();

        assert!(get_bee(db, bee_id).await.unwrap().is_none());
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
                ..Default::default()
            },
            ..Default::default()
        };
        let db = MockDbService::default();

        let bee_data = BeeData {
            id: 5,
            ..Default::default()
        };

        let bee_info = data_to_info(&config, &bee_data).unwrap();

        assert_eq!(bee_info.api_port, "1705");
        assert_eq!(bee_info.p2p_port, "1805");
        assert_eq!(bee_info.image, "bee-image:latest");
    }
}

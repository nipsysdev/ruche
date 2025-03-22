use std::env;

use anyhow::{anyhow, Result};

const NEIGHBORHOOD_API_URL: &'static str =
    "https://api.swarmscan.io/v1/network/neighborhoods/suggestion";

pub async fn get_neighborhood() -> Result<String> {
    let url = env::var("NEIGHBORHOOD_API_URL").unwrap_or_else(|_| NEIGHBORHOOD_API_URL.to_string());

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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

        let result = get_neighborhood().await.unwrap();

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

        let result = get_neighborhood().await;

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

        let result = get_neighborhood().await;

        assert!(result.is_err());
    }
}

use reqwest::Client;
use serde_json::json;
use std::time::Duration;

use crate::{
    config::DeploymentConfig,
    crypto::Encryptor,
    error::Error,
    types::{DeploymentResponse, VmConfig},
};

/// Client for interacting with the TEE deployment API
pub struct TeeClient {
    client: Client,
    config: DeploymentConfig,
}

impl TeeClient {
    /// Create a new TeeClient with the given configuration
    pub fn new(config: DeploymentConfig) -> Result<Self, Error> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(Error::HttpClient)?;

        Ok(Self { client, config })
    }

    /// Deploy a container to the TEE environment
    pub async fn deploy(&self) -> Result<DeploymentResponse, Error> {
        // Get or create VM configuration
        let vm_config = self.config.vm_config.clone().unwrap_or_else(|| VmConfig {
            name: format!("tee-deploy-{}", uuid::Uuid::new_v4()),
            compose_manifest: crate::types::ComposeManifest {
                name: "tee-deployment".to_string(),
                features: vec!["kms".to_string(), "tproxy-net".to_string()],
                docker_compose_file: self.config.docker_compose.clone(),
            },
            vcpu: 2,
            memory: 8192,
            disk_size: 40,
            teepod_id: self.config.teepod_id,
            image: self.config.image.clone(),
            advanced_features: crate::types::AdvancedFeatures {
                tproxy: true,
                kms: true,
                public_sys_info: true,
                public_logs: true,
                docker_config: crate::types::DockerConfig {
                    username: String::new(),
                    password: String::new(),
                    registry: None,
                },
                listed: false,
            },
        });

        // Get encryption public key
        let pubkey_response = self.get_pubkey(&vm_config).await?;

        // Encrypt environment variables
        let encryptor = Encryptor::new();
        let env_vars: Vec<_> = self
            .config
            .env_vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let encrypted_env = encryptor.encrypt_env_vars(
            &env_vars,
            &pubkey_response["app_env_encrypt_pubkey"]
                .as_str()
                .ok_or_else(|| Error::Api {
                    status_code: 500,
                    message: "Missing encryption public key".into(),
                })?,
        )?;

        // Create deployment
        let response = self
            .client
            .post(format!(
                "{}/cvms/from_cvm_configuration",
                self.config.api_url
            ))
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.config.api_key)
            .json(&json!({
                "vm_config": vm_config,
                "encrypted_env": encrypted_env,
                "app_env_encrypt_pubkey": pubkey_response["app_env_encrypt_pubkey"],
                "app_id_salt": pubkey_response["app_id_salt"],
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Api {
                status_code: response.status().as_u16(),
                message: response.text().await?,
            });
        }

        response
            .json::<DeploymentResponse>()
            .await
            .map_err(Error::HttpClient)
    }

    async fn get_pubkey(&self, vm_config: &VmConfig) -> Result<serde_json::Value, Error> {
        let response = self
            .client
            .post(format!(
                "{}/cvms/pubkey/from_cvm_configuration",
                self.config.api_url
            ))
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.config.api_key)
            .json(&vm_config)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Api {
                status_code: response.status().as_u16(),
                message: response.text().await?,
            });
        }

        response.json().await.map_err(Error::HttpClient)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // Helper function to create a test configuration
    fn create_test_config(api_url: String) -> DeploymentConfig {
        let mut env_vars = HashMap::new();
        env_vars.insert("TEST_KEY".to_string(), "test_value".to_string());
        env_vars.insert("ANOTHER_KEY".to_string(), "another_value".to_string());

        DeploymentConfig::new(
            "test_api_key".to_string(),
            "version: '3'".to_string(),
            env_vars,
            1,
            "test-image:latest".to_string(),
        )
        .with_api_url(api_url)
    }

    #[tokio::test]
    async fn test_successful_deployment_flow() {
        let mock_server = MockServer::start().await;

        // Mock the pubkey endpoint with validation
        Mock::given(method("POST"))
            .and(path("/cvms/pubkey/from_cvm_configuration"))
            .and(header("Content-Type", "application/json"))
            .and(header("x-api-key", "test_api_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "app_env_encrypt_pubkey": format!("0x{}", hex::encode([1u8; 32])),
                "app_id_salt": "test_salt"
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Mock the deployment endpoint with validation
        Mock::given(method("POST"))
            .and(path("/cvms/from_cvm_configuration"))
            .and(header("Content-Type", "application/json"))
            .and(header("x-api-key", "test_api_key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "test-deployment",
                "status": "pending",
                "details": {
                    "deployment_time": "2024-03-14T12:00:00Z"
                }
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = create_test_config(mock_server.uri());
        let client = TeeClient::new(config).unwrap();
        let result = client.deploy().await.unwrap();

        assert_eq!(result.id, "test-deployment");
        assert_eq!(result.status, "pending");
    }

    #[tokio::test]
    async fn test_api_error_handling() {
        let mock_server = MockServer::start().await;

        // Mock API error response
        Mock::given(method("POST"))
            .and(path("/cvms/pubkey/from_cvm_configuration"))
            .respond_with(ResponseTemplate::new(422).set_body_json(json!({
                "error": "Invalid configuration"
            })))
            .mount(&mock_server)
            .await;

        let config = create_test_config(mock_server.uri());
        let client = TeeClient::new(config).unwrap();
        let result = client.deploy().await;

        assert!(matches!(
            result,
            Err(Error::Api {
                status_code: 422,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn test_timeout_handling() {
        let mock_server = MockServer::start().await;

        // Mock delayed response beyond timeout
        Mock::given(method("POST"))
            .and(path("/cvms/pubkey/from_cvm_configuration"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(6)))
            .mount(&mock_server)
            .await;

        let config = create_test_config(mock_server.uri());
        let client = TeeClient::new(config).unwrap();
        let result = client.deploy().await;

        assert!(matches!(result, Err(Error::HttpClient(_))));
    }
}

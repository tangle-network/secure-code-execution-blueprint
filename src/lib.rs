use api::services::events::JobCalled;
use blueprint_sdk::config::GadgetConfiguration;
use blueprint_sdk::event_listeners::tangle::events::TangleEventListener;
use blueprint_sdk::event_listeners::tangle::services::{
    services_post_processor, services_pre_processor,
};
use blueprint_sdk::macros::contexts::{ServicesContext, TangleClientContext};
use blueprint_sdk::tangle_subxt::tangle_testnet_runtime::api;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;

#[derive(Clone, TangleClientContext, ServicesContext)]
pub struct ServiceContext {
    #[config]
    pub config: GadgetConfiguration,
    #[call_id]
    pub call_id: Option<u64>,
    pub code_exec_url: String,
    pub http_client: Client,
}

#[derive(Debug, Serialize)]
struct CodeExecutionRequest {
    language: String,
    code: String,
    input: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CodeExecutionResponse {
    stdout: String,
    stderr: String,
    execution_time: u64,
    memory_usage: u64,
}

/// Execute code in the specified language
#[blueprint_sdk::job(
    id = 0,
    params(language, code, input),
    result(_),
    event_listener(
        listener = TangleEventListener::<ServiceContext, JobCalled>,
        pre_processor = services_pre_processor,
        post_processor = services_post_processor,
    ),
)]
pub async fn execute_code(
    language: String,
    code: String,
    input: Option<String>,
    context: ServiceContext,
) -> Result<String, Error> {
    let request = CodeExecutionRequest {
        language,
        code,
        input,
    };

    let response = context
        .http_client
        .post(&format!("{}/execute", context.code_exec_url))
        .json(&request)
        .send()
        .await?
        .json::<CodeExecutionResponse>()
        .await?;

    if !response.stderr.is_empty() {
        return Err(Error::ExecutionError(response.stderr));
    }

    Ok(response.stdout)
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Execution error: {0}")]
    ExecutionError(String),
}

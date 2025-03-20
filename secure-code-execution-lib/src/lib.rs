use blueprint_sdk::extract::Context;
use blueprint_sdk::macros::context::{ServicesContext, TangleClientContext};
use blueprint_sdk::runner::config::BlueprintEnvironment;
use blueprint_sdk::tangle::extract::{TangleArgs3, TangleResult};
use serde::{Deserialize, Serialize};

// The job ID for execute_code
pub const EXECUTE_CODE_JOB_ID: u32 = 0;

#[derive(Default, Clone, TangleClientContext, ServicesContext)]
pub struct ServiceContext {
    #[config]
    pub config: BlueprintEnvironment,
    #[call_id]
    pub call_id: Option<u64>,
    pub code_exec_url: String,
    pub http_client: reqwest::Client,
}

impl ServiceContext {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct CodeExecutionRequest {
    language: String,
    code: String,
    input: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct CodeExecutionResponse {
    stdout: String,
    stderr: String,
    execution_time: u64,
    memory_usage: u64,
}

// Execute code in the specified language
pub async fn execute_code(
    Context(ctx): Context<ServiceContext>,
    TangleArgs3(language, code, input): TangleArgs3<String, String, Option<String>>,
) -> Result<TangleResult<()>, blueprint_sdk::Error> {
    let request = CodeExecutionRequest {
        language,
        code,
        input,
    };

    let response = ctx
        .http_client
        .post(&format!("{}/execute", ctx.code_exec_url))
        .json(&request)
        .send()
        .await
        .map_err(|e| blueprint_sdk::Error::Other(e.to_string()))?
        .json::<CodeExecutionResponse>()
        .await
        .map_err(|e| blueprint_sdk::Error::Other(e.to_string()))?;

    if !response.stderr.is_empty() {
        return Err(blueprint_sdk::Error::Other(response.stderr));
    }

    Ok(TangleResult(()))
}

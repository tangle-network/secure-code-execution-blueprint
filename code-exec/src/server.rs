use crate::{CodeExecutionService, Error, ExecutionRequest, ExecutionResult, Language};
use axum::{
    extract::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tracing::{error, info};

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteRequest {
    pub language: String,
    pub code: String,
    pub input: Option<String>,
    pub timeout: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ExecuteResponse {
    pub stdout: String,
    pub stderr: String,
    pub status: String,
    pub execution_time: u64,
    pub memory_usage: u64,
}

pub struct CodeExecutionServer {
    service: Arc<CodeExecutionService>,
    port: u16,
}

impl CodeExecutionServer {
    pub fn new(service: CodeExecutionService, port: u16) -> Self {
        Self {
            service: Arc::new(service),
            port,
        }
    }

    pub async fn run(&self) -> Result<(), Error> {
        let service = self.service.clone();

        let app = Router::new()
            .route("/health", get(|| async { "OK" }))
            .route(
                "/execute",
                post(move |Json(payload): Json<ExecuteRequest>| async move {
                    let language: Language = match payload.language.parse() {
                        Ok(l) => l,
                        Err(_) => {
                            return Json(ExecuteResponse {
                                stdout: String::new(),
                                stderr: "Invalid language".to_string(),
                                status: "error".to_string(),
                                execution_time: 0,
                                memory_usage: 0,
                            });
                        }
                    };
                    let request = ExecutionRequest {
                        language,
                        code: payload.code,
                        input: payload.input,
                        timeout: Duration::from_secs(payload.timeout.unwrap_or(30)),
                        dependencies: vec![],
                        env_vars: Default::default(),
                    };

                    match service.execute(request).await {
                        Ok(ExecutionResult {
                            stdout,
                            stderr,
                            status,
                            process_stats,
                        }) => Json(ExecuteResponse {
                            stdout,
                            stderr,
                            status: status.to_string(),
                            execution_time: process_stats.execution_time.as_millis() as u64,
                            memory_usage: process_stats.memory_usage,
                        }),
                        Err(e) => {
                            error!("Execution error: {}", e);
                            Json(ExecuteResponse {
                                stdout: String::new(),
                                stderr: e.to_string(),
                                status: "error".to_string(),
                                execution_time: 0,
                                memory_usage: 0,
                            })
                        }
                    }
                }),
            );

        let addr = format!("0.0.0.0:{}", self.port);
        info!("Starting code execution server on {}", addr);
        let listener = TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

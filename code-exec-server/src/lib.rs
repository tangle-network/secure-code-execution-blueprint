use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use code_exec::{
    CodeExecutionService, Dependency, ExecutionRequest, ExecutionResult, Language, ProcessStats,
    ResourceLimits,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use thiserror::Error;
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info};

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Invalid language: {0}")]
    InvalidLanguage(String),
    #[error("Execution error: {0}")]
    ExecutionError(#[from] code_exec::Error),
    #[error("Server error: {0}")]
    ServerError(String),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ServerError::InvalidLanguage(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ServerError::ExecutionError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ServerError::ServerError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExecuteRequest {
    pub language: String,
    pub code: String,
    pub input: Option<String>,
    pub timeout: Option<u64>,
    pub dependencies: Option<Vec<Dependency>>,
    pub env_vars: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecuteResponse {
    pub stdout: String,
    pub stderr: String,
    pub status: String,
    pub process_stats: ProcessStats,
}

#[derive(Clone)]
pub struct AppState {
    service: Arc<CodeExecutionService>,
}

pub async fn create_app(
    max_concurrent_executions: usize,
    resource_limits: ResourceLimits,
) -> Result<Router, ServerError> {
    let service = CodeExecutionService::new(max_concurrent_executions, resource_limits)
        .await
        .map_err(ServerError::ExecutionError)?;

    let state = AppState {
        service: Arc::new(service),
    };

    let cors = CorsLayer::permissive();

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/execute", post(execute))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    Ok(app)
}

pub async fn run_server(app: Router, addr: SocketAddr) -> Result<(), ServerError> {
    info!("Starting code execution server on {}", addr);
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| ServerError::ServerError(e.to_string()))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| ServerError::ServerError(e.to_string()))?;

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}

async fn execute(
    State(state): State<AppState>,
    Json(payload): Json<ExecuteRequest>,
) -> Result<Json<ExecuteResponse>, ServerError> {
    let language: Language = payload
        .language
        .parse()
        .map_err(|_| ServerError::InvalidLanguage(payload.language))?;

    let request = ExecutionRequest {
        language,
        code: payload.code,
        input: payload.input,
        timeout: Duration::from_secs(payload.timeout.unwrap_or(30)),
        dependencies: payload.dependencies.unwrap_or_default(),
        env_vars: payload.env_vars.unwrap_or_default(),
    };

    let result = state
        .service
        .execute(request)
        .await
        .map_err(ServerError::ExecutionError)?;

    Ok(Json(ExecuteResponse {
        stdout: result.stdout,
        stderr: result.stderr,
        status: result.status.to_string(),
        process_stats: result.process_stats,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_check() {
        let app = create_app(1, ResourceLimits::default())
            .await
            .expect("Failed to create app");

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_execute() {
        let app = create_app(1, ResourceLimits::default())
            .await
            .expect("Failed to create app");

        let request = ExecuteRequest {
            language: "python".to_string(),
            code: r#"print("Hello, World!")"#.to_string(),
            input: None,
            timeout: Some(5),
            dependencies: None,
            env_vars: None,
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/execute")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Get the response body and verify the execution result
        let limit = usize::MAX;
        let body = axum::body::to_bytes(response.into_body(), limit)
            .await
            .unwrap();
        let result: ExecuteResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(result.stdout.trim(), "Hello, World!");
        assert!(result.stderr.is_empty());
        assert_eq!(result.status, "success");
    }
}

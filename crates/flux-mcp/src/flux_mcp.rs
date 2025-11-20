use std::sync::Arc;

use rmcp::{
    ErrorData as McpErrorData, ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use tokio::sync::Mutex;

use crate::{
    diagnostics,
    flux_runner::{FluxRunner, VerificationReport, VerifyRepositoryArgs},
};

pub struct FluxMcp {
    runner: Arc<Mutex<FluxRunner>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl FluxMcp {
    pub fn new() -> Self {
        Self { runner: Arc::new(Mutex::new(FluxRunner::new())), tool_router: Self::tool_router() }
    }

    #[tool(description = "Run Flux verification on a repository and return results")]
    async fn verify_repository(
        &self,
        Parameters(args): Parameters<VerifyRepositoryArgs>,
    ) -> Result<CallToolResult, McpErrorData> {
        let runner = self.runner.lock().await;
        let result = runner.verify_repository(&args.repo_path).await;
        match result {
            Ok(report) => {
                let result_text = if report.success {
                    "Verification Succeeded".to_string()
                } else {
                    "Verification Failed".to_string()
                };
                let mut diagnostic_text: Vec<_> = report
                    .diagnostics
                    .iter()
                    .map(|diagnostic| Content::text(serde_json::to_string(diagnostic).unwrap()))
                    .collect();
                diagnostic_text.push(Content::text(result_text));
                Ok(CallToolResult::success(diagnostic_text))
            }
            Err(err) => {
                Err(McpErrorData::invalid_request(format!("Verification failed {err}"), None))
            }
        }
    }

    #[tool(description = "Get only the syntax errors from Flux verification")]
    async fn get_syntax_errors(
        &self,
        Parameters(args): Parameters<VerifyRepositoryArgs>,
    ) -> Result<CallToolResult, McpErrorData> {
        let runner = self.runner.lock().await;
        let result = runner.verify_repository(&args.repo_path).await;
        match result {
            Ok(VerificationReport { diagnostics, .. }) => {
                let syntax_errors = diagnostics::retain_only_syntax_errors(diagnostics);
                let result_text = format!("Found {} syntax errors", syntax_errors.len());
                let mut diagnostic_text: Vec<_> = syntax_errors
                    .iter()
                    .map(|diagnostic| Content::text(serde_json::to_string(diagnostic).unwrap()))
                    .collect();
                diagnostic_text.push(Content::text(result_text));
                Ok(CallToolResult::success(diagnostic_text))
            }
            Err(err) => {
                Err(McpErrorData::invalid_request(format!("Verification failed {err}"), None))
            }
        }
    }
}

#[tool_handler]
impl ServerHandler for FluxMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("This server exposes Flux verification tools".to_string()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

use std::process::ExitCode;

use ark_service::server;

#[tokio::main]
async fn main() -> ExitCode {
    match server::run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!(error = %e, "Service stopped");
            ExitCode::FAILURE
        }
    }
}

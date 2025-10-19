use ark_service::server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    server::run().await
}

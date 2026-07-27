#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    if let Err(error) = api::controlled_ai_gateway::run().await {
        tracing::error!(%error, "AI gateway terminated");
        std::process::exit(1);
    }
}

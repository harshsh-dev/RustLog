use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    rustlog::run().await
}

use normhub::core::Application;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();

    Application::new()?.run().await?;

    Ok(())
}

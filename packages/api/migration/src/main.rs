use migration::{establish_connection, run_migrations};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    // Get database URL from environment
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    println!("Connecting to database...");
    let pool = establish_connection(&database_url).await?;

    println!("Running migrations...");
    run_migrations(&pool).await?;

    println!("Migrations completed successfully!");

    Ok(())
}
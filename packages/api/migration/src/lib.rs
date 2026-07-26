use sqlx::{migrate::MigrateDatabase, postgres::PgPoolOptions, PgPool, Postgres};

pub async fn establish_connection(database_url: &str) -> Result<PgPool, sqlx::Error> {
    // Create database if it doesn't exist
    if !Postgres::database_exists(database_url).await.unwrap_or(false) {
        Postgres::create_database(database_url).await?;
        println!("Database created: {}", database_url);
    }

    // Create connection pool
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(database_url)
        .await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    // Run migrations from the migrations directory
    sqlx::migrate!("./migrations").run(pool).await.map_err(|e| sqlx::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))
}
use payment_service::{
    config::AppConfig,
    db::{connection::create_pool, migrations::run as run_migrations},
    routes::create_router,
    state::AppState,
};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let config = AppConfig::from_env()?;
    let address = config.socket_addr()?;
    let database_pool = create_pool(&config.database_url).await?;
    run_migrations(&database_pool).await?;
    let app = create_router(AppState::new(config, database_pool));

    let listener = TcpListener::bind(address).await?;

    println!("==================================");
    println!(" Payment Service Started");
    println!(" PostgreSQL connection established");
    println!(" Database migrations applied");
    println!(" Listening on http://{address}");
    println!("==================================");

    axum::serve(listener, app).await?;
    Ok(())
}

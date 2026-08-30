use payment_service::{
    config::AppConfig,
    db::{connection::create_pool, migrations::run as run_migrations},
    locks::RedisLock,
    routes::create_router,
    state::AppState,
    telemetry::init_tracing,
};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    init_tracing();

    let config = AppConfig::from_env()?;
    let address = config.socket_addr()?;
    let database_pool = create_pool(&config.database_url, config.database_max_connections).await?;
    run_migrations(&database_pool).await?;
    let redis_locks = RedisLock::connect(&config.redis_url, config.redis_lock_ttl).await?;
    let app = create_router(AppState::new(config, database_pool, redis_locks));

    let listener = TcpListener::bind(address).await?;

    info!(%address, "payment service started");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C signal handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM signal handler")
            .recv()
            .await;
    };

    #[cfg(unix)]
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    #[cfg(not(unix))]
    ctrl_c.await;

    info!("shutdown signal received; draining in-flight requests");
}

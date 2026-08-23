use payment_service::{config::AppConfig, routes::create_router, state::AppState};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::from_env()?;
    let address = config.socket_addr()?;
    let app = create_router(AppState::new(config));

    let listener = TcpListener::bind(address).await?;

    println!("==================================");
    println!(" Payment Service Started");
    println!(" Listening on http://{address}");
    println!("==================================");

    axum::serve(listener, app).await?;
    Ok(())
}

use std::sync::Arc;
use std::env;
use log::info;

mod protocol;
mod scoreboard;
mod web;

use scoreboard::ScoreboardController;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    // Get configuration from environment variables or use defaults
    let scoreboard_address = env::var("SCOREBOARD_ADDRESS")
        .unwrap_or_else(|_| "192.168.1.100:5200".to_string());
    let card_id = env::var("CARD_ID")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<u8>()
        .unwrap_or(1);
    let web_port = env::var("WEB_PORT")
        .unwrap_or_else(|_| "3030".to_string())
        .parse::<u16>()
        .unwrap_or(3030);

    info!("Starting HRUFC Scoreboard Server");
    info!("Scoreboard address: {}", scoreboard_address);
    info!("Card ID: {}", card_id);
    info!("Web server port: {}", web_port);

    // Create scoreboard controller
    let controller = Arc::new(ScoreboardController::new(scoreboard_address, card_id));

    // Initialize the scoreboard (connect and set up display)
    info!("Initializing scoreboard...");
    match controller.initialize().await {
        Ok(_) => info!("Scoreboard initialized successfully"),
        Err(e) => {
            log::error!("Failed to initialize scoreboard: {}. Will continue with web server.", e);
            // Continue running even if we can't connect to scoreboard initially
            // This allows the web interface to be accessible for configuration
        }
    }

    // Start connection monitoring in background
    let controller_monitor = controller.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(e) = controller_monitor.ensure_connection().await {
                log::warn!("Connection check failed: {}", e);
            }
        }
    });

    // Create web routes
    let routes = web::create_routes(controller);

    // Start web server
    info!("Starting web server on port {}", web_port);
    warp::serve(routes)
        .run(([0, 0, 0, 0], web_port))
        .await;

    Ok(())
}

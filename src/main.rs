use std::sync::Arc;
use std::env;
use log::info;

mod config;
mod protocol;
mod scoreboard;
mod web;

use config::Config;
use scoreboard::ScoreboardController;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    // Load configuration
    let config = Config::load().unwrap_or_else(|e| {
        log::warn!("Failed to load configuration: {}. Using defaults.", e);
        Config::default()
    });

    // Override config with environment variables if present
    let scoreboard_address = env::var("SCOREBOARD_ADDRESS")
        .unwrap_or_else(|_| config.scoreboard.address.clone());
    let card_id = env::var("CARD_ID")
        .unwrap_or_else(|_| config.scoreboard.card_id.to_string())
        .parse::<u8>()
        .unwrap_or(config.scoreboard.card_id);
    let web_port = env::var("WEB_PORT")
        .unwrap_or_else(|_| config.server.web_port.to_string())
        .parse::<u16>()
        .unwrap_or(config.server.web_port);
    let simulation_mode = env::var("SIMULATION_MODE")
        .unwrap_or_else(|_| config.server.simulation_mode.to_string())
        .parse::<bool>()
        .unwrap_or(config.server.simulation_mode);

    info!("Starting HRUFC Rugby Scoreboard Server");
    info!("Configuration loaded from config.yaml");
    info!("Simulation mode: {}", simulation_mode);
    info!("Scoreboard address: {}", scoreboard_address);
    info!("Card ID: {}", card_id);
    
    // Display web server access information
    let access_url = if web_port == 80 {
        format!("http://localhost/")
    } else {
        format!("http://localhost:{}/", web_port)
    };
    info!("Web server will start on port {}", web_port);
    info!("Access the scoreboard interface at: {}", access_url);

    // Create scoreboard controller
    let controller = Arc::new(ScoreboardController::new(
        scoreboard_address,
        card_id,
        simulation_mode,
        config.clone(),
    ));

    // Initialize the scoreboard (connect and set up display) - skip if simulation mode
    if !simulation_mode {
        info!("Initializing scoreboard connection...");
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
        let reconnect_interval = config.scoreboard.reconnect_interval_seconds;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(reconnect_interval));
            loop {
                interval.tick().await;
                if let Err(e) = controller_monitor.ensure_connection().await {
                    log::warn!("Connection check failed: {}", e);
                }
            }
        });
    } else {
        info!("Running in simulation mode - no physical scoreboard connection");
    }

    // Create web routes
    let routes = web::create_routes(controller);

    // Start web server
    info!("Web server started - Access at: {}", access_url);
    warp::serve(routes)
        .run(([0, 0, 0, 0], web_port))
        .await;

    Ok(())
}

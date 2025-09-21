use warp::{Filter, Rejection, Reply};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::convert::Infallible;
use log::{info, error};

use crate::scoreboard::ScoreboardController;

#[derive(Debug, Deserialize)]
pub struct TeamUpdate {
    pub home_team: String,
    pub away_team: String,
}

#[derive(Debug, Deserialize)]
pub struct ScoreUpdate {
    pub home_score: u16,
    pub away_score: u16,
}

#[derive(Debug, Deserialize)]
pub struct TimerUpdate {
    pub minutes: u8,
    pub seconds: u8,
}

#[derive(Debug, Deserialize)]
pub struct TeamAction {
    pub team: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigUpdate {
    pub web_port: Option<u16>,
    pub simulation_mode: Option<bool>,
    pub scoreboard_address: Option<String>,
    pub card_id: Option<u8>,
    pub try_points: Option<u16>,
    pub conversion_points: Option<u16>,
    pub penalty_points: Option<u16>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

// Helper function to create a JSON reply with proper type annotations
fn json_reply<T: serde::Serialize>(response: ApiResponse<T>) -> Result<warp::reply::Json, Infallible> {
    Ok(warp::reply::json(&response))
}

/// Create the web server routes
pub fn create_routes(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type"])
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE"]);

    let static_files = warp::path("static")
        .and(warp::fs::dir("static"));

    let index = warp::path::end()
        .and(warp::get())
        .and(warp::fs::file("static/index.html"));

    let config_page = warp::path("config")
        .and(warp::get())
        .and(warp::fs::file("static/config.html"));

    let api_routes = warp::path("api").and(
        get_status(controller.clone())
            .or(set_teams(controller.clone()))
            .or(set_scores(controller.clone()))
            .or(increment_home_score(controller.clone()))
            .or(increment_away_score(controller.clone()))
            .or(reset_scores(controller.clone()))
            .or(set_timer(controller.clone()))
            .or(start_timer(controller.clone()))
            .or(stop_timer(controller.clone()))
            .or(reset_timer(controller.clone()))
            .or(add_try(controller.clone()))
            .or(remove_try(controller.clone()))
            .or(add_conversion(controller.clone()))
            .or(remove_conversion(controller.clone()))
            .or(add_penalty(controller.clone()))
            .or(remove_penalty(controller.clone()))
            .or(add_penalty_try(controller.clone()))
            .or(start_first_half(controller.clone()))
            .or(start_second_half(controller.clone()))
            .or(end_period(controller.clone()))
            .or(get_config(controller.clone()))
            .or(update_config(controller.clone()))
    );

    index
        .or(config_page)
        .or(static_files)
        .or(api_routes)
        .with(cors)
}

/// GET /api/status
fn get_status(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("status")
        .and(warp::get())
        .and_then(move || {
            let controller = controller.clone();
            async move {
                let state = controller.get_state().await;
                let connected = controller.is_connected().await;
                let mut response_state = state;
                response_state.connected = connected;
                json_reply(ApiResponse::success(response_state))
            }
        })
}

/// POST /api/teams
fn set_teams(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("teams")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(move |update: TeamUpdate| {
            let controller = controller.clone();
            async move {
                match controller.set_teams(update.home_team, update.away_team).await {
                    Ok(_) => {
                        info!("Teams updated successfully");
                        json_reply(ApiResponse::success("Teams updated".to_string()))
                    }
                    Err(e) => {
                        error!("Failed to update teams: {}", e);
                        json_reply(ApiResponse::<String>::error(e.to_string()))
                    }
                }
            }
        })
}

/// POST /api/scores
fn set_scores(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("scores")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(move |update: ScoreUpdate| {
            let controller = controller.clone();
            async move {
                match controller.set_scores(update.home_score, update.away_score).await {
                    Ok(_) => {
                        info!("Scores updated successfully");
                        json_reply(ApiResponse::success("Scores updated".to_string()))
                    }
                    Err(e) => {
                        error!("Failed to update scores: {}", e);
                        json_reply(ApiResponse::<String>::error(e.to_string()))
                    }
                }
            }
        })
}

/// POST /api/scores/home/increment
fn increment_home_score(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("scores" / "home" / "increment")
        .and(warp::post())
        .and_then(move || {
            let controller = controller.clone();
            async move {
                match controller.increment_home_score().await {
                    Ok(_) => {
                        info!("Home score incremented");
                        json_reply(ApiResponse::success("Home score incremented".to_string()))
                    }
                    Err(e) => {
                        error!("Failed to increment home score: {}", e);
                        json_reply(ApiResponse::<String>::error(e.to_string()))
                    }
                }
            }
        })
}

/// POST /api/scores/away/increment
fn increment_away_score(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("scores" / "away" / "increment")
        .and(warp::post())
        .and_then(move || {
            let controller = controller.clone();
            async move {
                match controller.increment_away_score().await {
                    Ok(_) => {
                        info!("Away score incremented");
                        json_reply(ApiResponse::success("Away score incremented".to_string()))
                    }
                    Err(e) => {
                        error!("Failed to increment away score: {}", e);
                        json_reply(ApiResponse::<String>::error(e.to_string()))
                    }
                }
            }
        })
}

/// POST /api/scores/reset
fn reset_scores(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("scores" / "reset")
        .and(warp::post())
        .and_then(move || {
            let controller = controller.clone();
            async move {
                match controller.reset_scores().await {
                    Ok(_) => {
                        info!("Scores reset");
                        json_reply(ApiResponse::success("Scores reset".to_string()))
                    }
                    Err(e) => {
                        error!("Failed to reset scores: {}", e);
                        json_reply(ApiResponse::<String>::error(e.to_string()))
                    }
                }
            }
        })
}

/// POST /api/timer
fn set_timer(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("timer")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(move |update: TimerUpdate| {
            let controller = controller.clone();
            async move {
                match controller.set_timer(update.minutes, update.seconds).await {
                    Ok(_) => {
                        info!("Timer set to {}:{:02}", update.minutes, update.seconds);
                        json_reply(ApiResponse::success("Timer updated".to_string()))
                    }
                    Err(e) => {
                        error!("Failed to set timer: {}", e);
                        json_reply(ApiResponse::<String>::error(e.to_string()))
                    }
                }
            }
        })
}

/// POST /api/timer/start
fn start_timer(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("timer" / "start")
        .and(warp::post())
        .and_then(move || {
            let controller = controller.clone();
            async move {
                match controller.start_timer().await {
                    Ok(_) => {
                        info!("Timer started");
                        json_reply(ApiResponse::success("Timer started".to_string()))
                    }
                    Err(e) => {
                        error!("Failed to start timer: {}", e);
                        json_reply(ApiResponse::<String>::error(e.to_string()))
                    }
                }
            }
        })
}

/// POST /api/timer/stop
fn stop_timer(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("timer" / "stop")
        .and(warp::post())
        .and_then(move || {
            let controller = controller.clone();
            async move {
                match controller.stop_timer().await {
                    Ok(_) => {
                        info!("Timer stopped");
                        json_reply(ApiResponse::success("Timer stopped".to_string()))
                    }
                    Err(e) => {
                        error!("Failed to stop timer: {}", e);
                        json_reply(ApiResponse::<String>::error(e.to_string()))
                    }
                }
            }
        })
}

/// POST /api/timer/reset
fn reset_timer(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("timer" / "reset")
        .and(warp::post())
        .and_then(move || {
            let controller = controller.clone();
            async move {
                match controller.reset_timer().await {
                    Ok(_) => {
                        info!("Timer reset");
                        json_reply(ApiResponse::success("Timer reset".to_string()))
                    }
                    Err(e) => {
                        error!("Failed to reset timer: {}", e);
                        json_reply(ApiResponse::<String>::error(e.to_string()))
                    }
                }
            }
        })
}/// POST /api/rugby/try
fn add_try(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("rugby" / "try")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(move |team_action: TeamAction| {
            let controller = controller.clone();
            async move {
                match controller.add_try(&team_action.team).await {
                    Ok(_) => {
                        info!("Try added for team: {}", team_action.team);
                        json_reply(ApiResponse::success(format!("Try added for {}", team_action.team)))
                    }
                    Err(e) => {
                        error!("Failed to add try: {}", e);
                        json_reply(ApiResponse::<String>::error(e.to_string()))
                    }
                }
            }
        })
}

/// DELETE /api/rugby/try
fn remove_try(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("rugby" / "try")
        .and(warp::delete())
        .and(warp::body::json())
        .and_then(move |team_action: TeamAction| {
            let controller = controller.clone();
            async move {
                match controller.remove_try(&team_action.team).await {
                    Ok(_) => {
                        info!("Try removed for team: {}", team_action.team);
                        json_reply(ApiResponse::success(format!("Try removed for {}", team_action.team)))
                    }
                    Err(e) => {
                        error!("Failed to remove try: {}", e);
                        json_reply(ApiResponse::<String>::error(e.to_string()))
                    }
                }
            }
        })
}

/// POST /api/rugby/conversion
fn add_conversion(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("rugby" / "conversion")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(move |team_action: TeamAction| {
            let controller = controller.clone();
            async move {
                match controller.add_conversion(&team_action.team).await {
                    Ok(_) => {
                        info!("Conversion added for team: {}", team_action.team);
                        json_reply(ApiResponse::success(format!("Conversion added for {}", team_action.team)))
                    }
                    Err(e) => {
                        error!("Failed to add conversion: {}", e);
                        json_reply(ApiResponse::<String>::error(e.to_string()))
                    }
                }
            }
        })
}

/// POST /api/rugby/penalty
fn add_penalty(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("rugby" / "penalty")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(move |team_action: TeamAction| {
            let controller = controller.clone();
            async move {
                match controller.add_penalty(&team_action.team).await {
                    Ok(_) => {
                        info!("Penalty added for team: {}", team_action.team);
                        json_reply(ApiResponse::success(format!("Penalty added for {}", team_action.team)))
                    }
                    Err(e) => {
                        error!("Failed to add penalty: {}", e);
                        json_reply(ApiResponse::<String>::error(e.to_string()))
                    }
                }
            }
        })
}

/// DELETE /api/rugby/conversion
fn remove_conversion(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("rugby" / "conversion")
        .and(warp::delete())
        .and(warp::body::json())
        .and_then(move |team_action: TeamAction| {
            let controller = controller.clone();
            async move {
                match controller.remove_conversion(&team_action.team).await {
                    Ok(_) => {
                        info!("Conversion removed for team: {}", team_action.team);
                        json_reply(ApiResponse::success(format!("Conversion removed for {}", team_action.team)))
                    }
                    Err(e) => {
                        error!("Failed to remove conversion: {}", e);
                        json_reply(ApiResponse::<String>::error(e.to_string()))
                    }
                }
            }
        })
}

/// DELETE /api/rugby/penalty
fn remove_penalty(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("rugby" / "penalty")
        .and(warp::delete())
        .and(warp::body::json())
        .and_then(move |team_action: TeamAction| {
            let controller = controller.clone();
            async move {
                match controller.remove_penalty(&team_action.team).await {
                    Ok(_) => {
                        info!("Penalty removed for team: {}", team_action.team);
                        json_reply(ApiResponse::success(format!("Penalty removed for {}", team_action.team)))
                    }
                    Err(e) => {
                        error!("Failed to remove penalty: {}", e);
                        json_reply(ApiResponse::<String>::error(e.to_string()))
                    }
                }
            }
        })
}

/// POST /api/rugby/penalty-try
fn add_penalty_try(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("rugby" / "penalty-try")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(move |team_action: TeamAction| {
            let controller = controller.clone();
            async move {
                match controller.add_penalty_try(&team_action.team).await {
                    Ok(_) => {
                        info!("Penalty try added for team: {}", team_action.team);
                        json_reply(ApiResponse::success(format!("Penalty try added for {}", team_action.team)))
                    }
                    Err(e) => {
                        error!("Failed to add penalty try: {}", e);
                        json_reply(ApiResponse::<String>::error(e.to_string()))
                    }
                }
            }
        })
}

/// POST /api/rugby/start-first-half
fn start_first_half(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("rugby" / "start-first-half")
        .and(warp::post())
        .and_then(move || {
            let controller = controller.clone();
            async move {
                match controller.start_first_half().await {
                    Ok(_) => {
                        info!("First half started");
                        json_reply(ApiResponse::success("First half started".to_string()))
                    }
                    Err(e) => {
                        error!("Failed to start first half: {}", e);
                        json_reply(ApiResponse::<String>::error(e.to_string()))
                    }
                }
            }
        })
}

/// POST /api/rugby/start-second-half
fn start_second_half(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("rugby" / "start-second-half")
        .and(warp::post())
        .and_then(move || {
            let controller = controller.clone();
            async move {
                match controller.start_second_half().await {
                    Ok(_) => {
                        info!("Second half started");
                        json_reply(ApiResponse::success("Second half started".to_string()))
                    }
                    Err(e) => {
                        error!("Failed to start second half: {}", e);
                        json_reply(ApiResponse::<String>::error(e.to_string()))
                    }
                }
            }
        })
}

/// POST /api/rugby/end-period
fn end_period(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("rugby" / "end-period")
        .and(warp::post())
        .and_then(move || {
            let controller = controller.clone();
            async move {
                match controller.end_period().await {
                    Ok(_) => {
                        info!("Period ended");
                        json_reply(ApiResponse::success("Period ended".to_string()))
                    }
                    Err(e) => {
                        error!("Failed to end period: {}", e);
                        json_reply(ApiResponse::<String>::error(e.to_string()))
                    }
                }
            }
        })
}

/// GET /api/config
fn get_config(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("config")
        .and(warp::get())
        .and_then(move || {
            let controller = controller.clone();
            async move {
                // Get current config from controller
                json_reply(ApiResponse::success("Configuration access not yet implemented".to_string()))
            }
        })
}

/// POST /api/config
fn update_config(
    controller: Arc<ScoreboardController>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("config")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(move |config_update: ConfigUpdate| {
            let controller = controller.clone();
            async move {
                // Update config through controller
                json_reply(ApiResponse::success("Configuration update not yet implemented".to_string()))
            }
        })
}
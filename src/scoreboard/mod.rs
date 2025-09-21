use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};
use anyhow::Result;
use log::{info, error};

use crate::config::Config;
use crate::protocol::{
    ScoreboardClient, Command, TimeCommand, DisplayCommand, 
    ScoreboardLayout, Color, windows
};

/// High-level scoreboard controller
#[derive(Clone)]
pub struct ScoreboardController {
    client: Arc<Mutex<Option<ScoreboardClient>>>,
    layout: ScoreboardLayout,
    state: Arc<Mutex<ScoreboardState>>,
    simulation_mode: bool,
    config: Config,
    timer_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScoreboardState {
    pub home_team: String,
    pub away_team: String,
    pub home_score: u16,
    pub away_score: u16,
    pub timer_minutes: u8,
    pub timer_seconds: u8,
    pub timer_running: bool,
    pub connected: bool,
    pub simulation_mode: bool,
    pub current_period: u8, // 1 = first half, 2 = second half, 0 = before game/halftime
    pub period_time_remaining: u16, // seconds remaining in current period
}

impl Default for ScoreboardState {
    fn default() -> Self {
        Self {
            home_team: "HOME".to_string(),
            away_team: "AWAY".to_string(),
            home_score: 0,
            away_score: 0,
            timer_minutes: 0,
            timer_seconds: 0,
            timer_running: false,
            connected: false,
            simulation_mode: false,
            current_period: 0,
            period_time_remaining: 0,
        }
    }
}

impl ScoreboardController {
    /// Create a new scoreboard controller
    pub fn new(address: String, card_id: u8, simulation_mode: bool, config: Config) -> Self {
        let client = if simulation_mode {
            None
        } else {
            Some(ScoreboardClient::new(address, card_id))
        };
        let layout = ScoreboardLayout::standard_224x32();
        
        Self {
            client: Arc::new(Mutex::new(client)),
            layout,
            state: Arc::new(Mutex::new({
                let mut state = ScoreboardState::default();
                state.simulation_mode = simulation_mode;
                state.connected = simulation_mode; // In simulation mode, always "connected"
                state
            })),
            simulation_mode,
            config,
            timer_task: Arc::new(Mutex::new(None)),
        }
    }

    /// Initialize the scoreboard display
    pub async fn initialize(&self) -> Result<()> {
        if self.simulation_mode {
            info!("Initializing scoreboard display in simulation mode...");
            {
                let mut state = self.state.lock().await;
                state.connected = true;
            }
            info!("Scoreboard initialized successfully (simulation mode)");
            return Ok(());
        }
        
        info!("Initializing scoreboard display...");
        
        let mut client_option = self.client.lock().await;
        let client = client_option.as_mut()
            .ok_or_else(|| anyhow::anyhow!("No client available in non-simulation mode"))?;
        
        // Connect to scoreboard
        client.connect().await?;
        
        // Create windows
        let windows = self.layout.all_windows();
        let create_cmd = Command::DisplayMessage(DisplayCommand::CreateWindows(windows));
        client.send_command(create_cmd).await?;
        
        // Update connection status
        {
            let mut state = self.state.lock().await;
            state.connected = true;
        }
        
        // Display initial content
        self.update_display().await?;
        
        info!("Scoreboard initialized successfully");
        Ok(())
    }

    /// Update the entire display
    pub async fn update_display(&self) -> Result<()> {
        if self.simulation_mode {
            // In simulation mode, just log the state
            let state = self.state.lock().await;
            let period_info = match state.current_period {
                1 => format!("First Half ({}:{:02} remaining)", state.period_time_remaining / 60, state.period_time_remaining % 60),
                2 => format!("Second Half ({}:{:02} remaining)", state.period_time_remaining / 60, state.period_time_remaining % 60),
                _ => "Pre-game/Halftime".to_string(),
            };
            info!("Simulation display update: {} {} - {} {}, Timer: {:02}:{:02} {}, Period: {}",
                state.home_team, state.home_score,
                state.away_team, state.away_score,
                state.timer_minutes, state.timer_seconds,
                if state.timer_running { "(Running)" } else { "(Stopped)" },
                period_info
            );
            return Ok(());
        }
        
        let state = self.state.lock().await.clone();
        
        let mut client_option = self.client.lock().await;
        let client = client_option.as_mut()
            .ok_or_else(|| anyhow::anyhow!("No client available in non-simulation mode"))?;
        
        // Update team names
        self.send_text_command(client, windows::HOME_NAME, &state.home_team, Color::WHITE).await?;
        self.send_text_command(client, windows::AWAY_NAME, &state.away_team, Color::WHITE).await?;
        
        // Update scores
        self.send_text_command(client, windows::HOME_SCORE, &state.home_score.to_string(), Color::GREEN).await?;
        self.send_text_command(client, windows::AWAY_SCORE, &state.away_score.to_string(), Color::GREEN).await?;
        
        // Update timer display
        let timer_text = format!("{:02}:{:02}", state.timer_minutes, state.timer_seconds);
        let timer_color = if state.timer_running { Color::RED } else { Color::WHITE };
        self.send_text_command(client, windows::TIMER, &timer_text, timer_color).await?;
        
        Ok(())
    }

    /// Helper method to send text to a window
    async fn send_text_command(&self, client: &mut ScoreboardClient, window_id: u8, text: &str, color: Color) -> Result<()> {
        let cmd = Command::DisplayMessage(DisplayCommand::SendPureText {
            window_id,
            text: text.to_string(),
            color,
        });
        client.send_command(cmd).await?;
        Ok(())
    }

    /// Set team names
    pub async fn set_teams(&self, home_team: String, away_team: String) -> Result<()> {
        {
            let mut state = self.state.lock().await;
            state.home_team = home_team;
            state.away_team = away_team;
        }
        
        self.update_display().await
    }

    /// Set scores
    pub async fn set_scores(&self, home_score: u16, away_score: u16) -> Result<()> {
        {
            let mut state = self.state.lock().await;
            state.home_score = home_score;
            state.away_score = away_score;
        }
        
        self.update_display().await
    }

    /// Increment home score
    pub async fn increment_home_score(&self) -> Result<()> {
        let current_score = {
            let state = self.state.lock().await;
            state.home_score
        };
        
        self.set_scores(current_score + 1, self.get_away_score().await).await
    }

    /// Increment away score
    pub async fn increment_away_score(&self) -> Result<()> {
        let current_score = {
            let state = self.state.lock().await;
            state.away_score
        };
        
        self.set_scores(self.get_home_score().await, current_score + 1).await
    }

    /// Reset scores
    pub async fn reset_scores(&self) -> Result<()> {
        self.set_scores(0, 0).await
    }

    /// Set timer
    pub async fn set_timer(&self, minutes: u8, seconds: u8) -> Result<()> {
        {
            let mut state = self.state.lock().await;
            state.timer_minutes = minutes;
            state.timer_seconds = seconds;
        }
        
        if !self.simulation_mode {
            // Send time command to scoreboard
            let mut client_option = self.client.lock().await;
            let client = client_option.as_mut()
                .ok_or_else(|| anyhow::anyhow!("No client available in non-simulation mode"))?;
            let cmd = Command::TimeControl(TimeCommand::Set {
                hours: 0,
                minutes,
                seconds,
            });
            client.send_command(cmd).await?;
        }
        
        self.update_display().await
    }

    /// Start timer
    pub async fn start_timer(&self) -> Result<()> {
        {
            let mut state = self.state.lock().await;
            state.timer_running = true;
        }
        
        if self.simulation_mode {
            // Start simulation timer task
            let state_clone = self.state.clone();
            let timer_task = tokio::spawn(async move {
                let mut interval = interval(Duration::from_secs(1));
                
                loop {
                    interval.tick().await;
                    
                    let (timer_minutes, timer_seconds, should_break) = {
                        let mut state = state_clone.lock().await;
                        if !state.timer_running {
                            return;
                        }
                        
                        // Increment timer
                        state.timer_seconds += 1;
                        if state.timer_seconds >= 60 {
                            state.timer_seconds = 0;
                            state.timer_minutes += 1;
                            if state.timer_minutes >= 100 {
                                // Cap at 99:59 to prevent overflow
                                state.timer_minutes = 99;
                                state.timer_seconds = 59;
                            }
                        }
                        
                        // Update period time remaining if in a period
                        if state.current_period > 0 && state.period_time_remaining > 0 {
                            state.period_time_remaining -= 1;
                            if state.period_time_remaining == 0 {
                                info!("Period {} ended", state.current_period);
                                // Timer keeps running but period ends
                            }
                        }
                        
                        (state.timer_minutes, state.timer_seconds, !state.timer_running)
                    };
                    
                    if should_break {
                        break;
                    }
                    
                    // Log every 10 seconds to avoid spam
                    if timer_seconds % 10 == 0 {
                        info!("Timer: {:02}:{:02}", timer_minutes, timer_seconds);
                    }
                }
                
                info!("Simulation timer task ended");
            });
            
            let mut timer_task_handle = self.timer_task.lock().await;
            *timer_task_handle = Some(timer_task);
        } else {
            let mut client_option = self.client.lock().await;
            let client = client_option.as_mut()
                .ok_or_else(|| anyhow::anyhow!("No client available in non-simulation mode"))?;
            let cmd = Command::TimeControl(TimeCommand::StartStop(true));
            client.send_command(cmd).await?;
        }
        
        self.update_display().await
    }

    /// Stop timer
    pub async fn stop_timer(&self) -> Result<()> {
        {
            let mut state = self.state.lock().await;
            state.timer_running = false;
        }
        
        if self.simulation_mode {
            // Stop the simulation timer task
            let mut timer_task_handle = self.timer_task.lock().await;
            if let Some(task) = timer_task_handle.take() {
                task.abort();
            }
        } else {
            let mut client_option = self.client.lock().await;
            let client = client_option.as_mut()
                .ok_or_else(|| anyhow::anyhow!("No client available in non-simulation mode"))?;
            let cmd = Command::TimeControl(TimeCommand::StartStop(false));
            client.send_command(cmd).await?;
        }
        
        self.update_display().await
    }

    /// Reset timer
    pub async fn reset_timer(&self) -> Result<()> {
        self.set_timer(0, 0).await?;
        self.stop_timer().await
    }

    /// Start first half (40 minutes by default)
    pub async fn start_first_half(&self) -> Result<()> {
        let first_half_minutes = self.config.rugby.first_half_minutes;
        {
            let mut state = self.state.lock().await;
            state.current_period = 1;
            state.period_time_remaining = (first_half_minutes as u16) * 60;
            state.timer_minutes = 0;
            state.timer_seconds = 0;
        }
        info!("Starting first half - {} minutes", first_half_minutes);
        self.start_timer().await
    }

    /// Start second half (40 minutes by default, timer continues from first half)
    pub async fn start_second_half(&self) -> Result<()> {
        let second_half_minutes = self.config.rugby.second_half_minutes;
        {
            let mut state = self.state.lock().await;
            state.current_period = 2;
            state.period_time_remaining = (second_half_minutes as u16) * 60;
            // Timer continues from where first half ended (typically around 40:00)
        }
        info!("Starting second half - {} minutes", second_half_minutes);
        self.start_timer().await
    }

    /// End current period (halftime or full time)
    pub async fn end_period(&self) -> Result<()> {
        {
            let mut state = self.state.lock().await;
            if state.current_period == 1 {
                info!("Halftime");
            } else if state.current_period == 2 {
                info!("Full time");
            }
            state.current_period = 0;
            state.period_time_remaining = 0;
        }
        self.stop_timer().await
    }

    // Rugby scoring methods

    /// Add a try to the specified team (5 points)
    pub async fn add_try(&self, team: &str) -> Result<()> {
        let try_points = self.config.rugby.try_points;
        match team.to_lowercase().as_str() {
            "home" => {
                let current_score = self.get_home_score().await;
                self.set_scores(current_score + try_points, self.get_away_score().await).await
            }
            "away" => {
                let current_score = self.get_away_score().await;
                self.set_scores(self.get_home_score().await, current_score + try_points).await
            }
            _ => Err(anyhow::anyhow!("Invalid team: {}", team)),
        }
    }

    /// Remove a try from the specified team (subtract 5 points)
    pub async fn remove_try(&self, team: &str) -> Result<()> {
        let try_points = self.config.rugby.try_points;
        match team.to_lowercase().as_str() {
            "home" => {
                let current_score = self.get_home_score().await;
                let new_score = if current_score >= try_points {
                    current_score - try_points
                } else {
                    0
                };
                self.set_scores(new_score, self.get_away_score().await).await
            }
            "away" => {
                let current_score = self.get_away_score().await;
                let new_score = if current_score >= try_points {
                    current_score - try_points
                } else {
                    0
                };
                self.set_scores(self.get_home_score().await, new_score).await
            }
            _ => Err(anyhow::anyhow!("Invalid team: {}", team)),
        }
    }

    /// Add a conversion to the specified team (2 points)
    pub async fn add_conversion(&self, team: &str) -> Result<()> {
        let conversion_points = self.config.rugby.conversion_points;
        match team.to_lowercase().as_str() {
            "home" => {
                let current_score = self.get_home_score().await;
                self.set_scores(current_score + conversion_points, self.get_away_score().await).await
            }
            "away" => {
                let current_score = self.get_away_score().await;
                self.set_scores(self.get_home_score().await, current_score + conversion_points).await
            }
            _ => Err(anyhow::anyhow!("Invalid team: {}", team)),
        }
    }

    /// Add a penalty to the specified team (3 points)
    pub async fn add_penalty(&self, team: &str) -> Result<()> {
        let penalty_points = self.config.rugby.penalty_points;
        match team.to_lowercase().as_str() {
            "home" => {
                let current_score = self.get_home_score().await;
                self.set_scores(current_score + penalty_points, self.get_away_score().await).await
            }
            "away" => {
                let current_score = self.get_away_score().await;
                self.set_scores(self.get_home_score().await, current_score + penalty_points).await
            }
            _ => Err(anyhow::anyhow!("Invalid team: {}", team)),
        }
    }

    /// Remove a conversion from the specified team (-2 points)
    pub async fn remove_conversion(&self, team: &str) -> Result<()> {
        let conversion_points = self.config.rugby.conversion_points;
        match team.to_lowercase().as_str() {
            "home" => {
                let current_score = self.get_home_score().await;
                let new_score = if current_score >= conversion_points {
                    current_score - conversion_points
                } else {
                    0
                };
                self.set_scores(new_score, self.get_away_score().await).await
            }
            "away" => {
                let current_score = self.get_away_score().await;
                let new_score = if current_score >= conversion_points {
                    current_score - conversion_points
                } else {
                    0
                };
                self.set_scores(self.get_home_score().await, new_score).await
            }
            _ => Err(anyhow::anyhow!("Invalid team: {}", team)),
        }
    }

    /// Remove a penalty from the specified team (-3 points)
    pub async fn remove_penalty(&self, team: &str) -> Result<()> {
        let penalty_points = self.config.rugby.penalty_points;
        match team.to_lowercase().as_str() {
            "home" => {
                let current_score = self.get_home_score().await;
                let new_score = if current_score >= penalty_points {
                    current_score - penalty_points
                } else {
                    0
                };
                self.set_scores(new_score, self.get_away_score().await).await
            }
            "away" => {
                let current_score = self.get_away_score().await;
                let new_score = if current_score >= penalty_points {
                    current_score - penalty_points
                } else {
                    0
                };
                self.set_scores(self.get_home_score().await, new_score).await
            }
            _ => Err(anyhow::anyhow!("Invalid team: {}", team)),
        }
    }

    /// Add a penalty try to the specified team (7 points - try + conversion)
    pub async fn add_penalty_try(&self, team: &str) -> Result<()> {
        let penalty_try_points = 7; // Standard penalty try is 7 points (try + conversion combined)
        match team.to_lowercase().as_str() {
            "home" => {
                let current_score = self.get_home_score().await;
                self.set_scores(current_score + penalty_try_points, self.get_away_score().await).await
            }
            "away" => {
                let current_score = self.get_away_score().await;
                self.set_scores(self.get_home_score().await, current_score + penalty_try_points).await
            }
            _ => Err(anyhow::anyhow!("Invalid team: {}", team)),
        }
    }

    /// Get current state
    pub async fn get_state(&self) -> ScoreboardState {
        self.state.lock().await.clone()
    }

    /// Get home score
    pub async fn get_home_score(&self) -> u16 {
        self.state.lock().await.home_score
    }

    /// Get away score  
    pub async fn get_away_score(&self) -> u16 {
        self.state.lock().await.away_score
    }

    /// Check connection status
    pub async fn is_connected(&self) -> bool {
        if self.simulation_mode {
            return self.state.lock().await.connected;
        }
        
        let client_option = self.client.lock().await;
        match client_option.as_ref() {
            Some(client) => client.is_connected(),
            None => false,
        }
    }

    /// Ensure connection and update status
    pub async fn ensure_connection(&self) -> Result<bool> {
        if self.simulation_mode {
            return Ok(self.state.lock().await.connected);
        }
        
        let mut client_option = self.client.lock().await;
        let client = client_option.as_mut()
            .ok_or_else(|| anyhow::anyhow!("No client available in non-simulation mode"))?;
            
        let result = client.ensure_connection().await;
        
        let connected = result.is_ok() && client.is_connected();
        {
            let mut state = self.state.lock().await;
            state.connected = connected;
        }
        
        Ok(connected)
    }
}
use bytes::{BufMut, BytesMut};

/// Commands for the CPower control card
#[derive(Debug, Clone)]
pub enum Command {
    /// Restart Hardware (0x2d)
    RestartHardware,
    /// Brightness Control (0x46)
    BrightnessControl { query: bool, brightness: Option<u8> },
    /// Time Control (0x47)
    TimeControl(TimeCommand),
    /// Query Version Info (0x4b)  
    QueryVersion,
    /// Power On/Off Control (0x76)
    PowerControl { query: bool, power_on: Option<bool> },
    /// Display Messages (0x7b)
    DisplayMessage(DisplayCommand),
}

#[derive(Debug, Clone)]
pub enum TimeCommand {
    /// Query current time
    Query,
    /// Set time (hours, minutes, seconds)
    Set { hours: u8, minutes: u8, seconds: u8 },
    /// Start/Stop timer (true = start, false = stop)
    StartStop(bool),
}

#[derive(Debug, Clone)]
pub enum DisplayCommand {
    /// Create windows for display
    CreateWindows(Vec<WindowData>),
    /// Send text to a window
    SendText { window_id: u8, text: String, color: Color },
    /// Send pure text to window (simplified)
    SendPureText { window_id: u8, text: String, color: Color },
    /// Display time in window
    DisplayTime { window_id: u8 },
}

#[derive(Debug, Clone)]
pub struct WindowData {
    pub x: u16,
    pub y: u16, 
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Color {
    pub const RED: Color = Color { red: 255, green: 0, blue: 0 };
    pub const GREEN: Color = Color { red: 0, green: 255, blue: 0 };
    pub const BLUE: Color = Color { red: 0, green: 0, blue: 255 };
    pub const WHITE: Color = Color { red: 255, green: 255, blue: 255 };
    pub const BLACK: Color = Color { red: 0, green: 0, blue: 0 };
}

impl Command {
    /// Encode command into bytes for packet data
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Command::RestartHardware => {
                vec![0x2d, 0x01, 0x00]
            },
            Command::TimeControl(time_cmd) => {
                match time_cmd {
                    TimeCommand::Query => vec![0x47, 0x01, 0x01],
                    TimeCommand::Set { hours, minutes, seconds } => {
                        vec![0x47, 0x01, 0x02, *hours, *minutes, *seconds]
                    },
                    TimeCommand::StartStop(start) => {
                        // This might need adjustment based on actual protocol
                        vec![0x47, 0x01, if *start { 0x03 } else { 0x04 }]
                    },
                }
            },
            Command::QueryVersion => {
                vec![0x4b, 0x01]
            },
            Command::PowerControl { query, power_on } => {
                if *query {
                    vec![0x76, 0x01]
                } else {
                    vec![0x76, 0x01, if power_on.unwrap_or(false) { 0x01 } else { 0x00 }]
                }
            },
            Command::DisplayMessage(display_cmd) => {
                match display_cmd {
                    DisplayCommand::CreateWindows(windows) => {
                        let mut data = vec![0x7b, 0x01]; // Command + Response request
                        
                        // Data length (will be calculated)
                        let data_length = 3 + (windows.len() * 8); // subcommand + packet info + window count + window data
                        data.extend_from_slice(&(data_length as u16).to_le_bytes());
                        
                        data.extend_from_slice(&[0x00, 0x00]); // Packet ID, Max Packet ID
                        data.push(0x01); // Subcommand: Create Windows
                        data.push(windows.len() as u8); // Number of windows
                        
                        for window in windows {
                            data.extend_from_slice(&window.x.to_le_bytes());
                            data.extend_from_slice(&window.y.to_le_bytes());
                            data.extend_from_slice(&window.width.to_le_bytes());
                            data.extend_from_slice(&window.height.to_le_bytes());
                        }
                        data
                    },
                    DisplayCommand::SendPureText { window_id, text, color } => {
                        let mut data = vec![0x7b, 0x01]; // Command + Response request
                        
                        // Data length
                        let text_bytes = text.as_bytes();
                        let data_length = 3 + 7 + text_bytes.len(); // subcommand + params + text length
                        data.extend_from_slice(&(data_length as u16).to_le_bytes());
                        
                        data.extend_from_slice(&[0x00, 0x00]); // Packet ID, Max Packet ID
                        data.push(0x12); // Subcommand: Send Pure Text
                        data.push(*window_id); // Target window
                        data.push(0x00); // Display mode (instant)
                        data.push(0x04); // Alignment (centered vertically, left justified)
                        data.push(0x01); // Speed (fastest)
                        data.extend_from_slice(&[0x00, 0x00]); // Display time (permanent)
                        data.push(0x02); // Font size
                        data.push(color.red);
                        data.push(color.green);
                        data.push(color.blue);
                        data.extend_from_slice(text_bytes);
                        data.push(0x00); // Null terminator
                        data
                    },
                    DisplayCommand::DisplayTime { window_id } => {
                        let mut data = vec![0x7b, 0x01]; // Command + Response request
                        data.extend_from_slice(&[0x06, 0x00]); // Data length
                        data.extend_from_slice(&[0x00, 0x00]); // Packet ID, Max Packet ID  
                        data.push(0x05); // Subcommand: Display Time
                        data.push(*window_id); // Target window
                        data.push(0x00); // Display mode
                        data.push(0x04); // Alignment
                        data.push(0x01); // Speed
                        data.push(0x02); // Font size
                        data
                    },
                    _ => vec![], // Other display commands not implemented yet
                }
            },
            _ => vec![],
        }
    }
}

/// Common scoreboard window layout for football/rugby
#[derive(Clone)]
pub struct ScoreboardLayout {
    pub home_name: WindowData,
    pub home_score: WindowData, 
    pub away_name: WindowData,
    pub away_score: WindowData,
    pub timer: WindowData,
}

impl ScoreboardLayout {
    /// Create a standard layout for a 224x32 pixel display
    pub fn standard_224x32() -> Self {
        Self {
            home_name: WindowData { x: 0, y: 0, width: 96, height: 16 },
            home_score: WindowData { x: 96, y: 0, width: 32, height: 16 },
            away_name: WindowData { x: 0, y: 16, width: 96, height: 16 },
            away_score: WindowData { x: 96, y: 16, width: 32, height: 16 },
            timer: WindowData { x: 128, y: 0, width: 96, height: 32 },
        }
    }

    /// Get all windows as a vector for creating
    pub fn all_windows(&self) -> Vec<WindowData> {
        vec![
            self.home_name.clone(),
            self.home_score.clone(),
            self.away_name.clone(),
            self.away_score.clone(),
            self.timer.clone(),
        ]
    }
}

/// Window IDs for the standard scoreboard layout
pub mod windows {
    pub const HOME_NAME: u8 = 0;
    pub const HOME_SCORE: u8 = 1;
    pub const AWAY_NAME: u8 = 2;
    pub const AWAY_SCORE: u8 = 3;
    pub const TIMER: u8 = 4;
}
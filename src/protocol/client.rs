use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{Duration, sleep, timeout};
use log::{info, warn, error, debug};
use anyhow::{Result, Context};

use crate::protocol::{EthernetPacket, Command};

/// TCP client for communicating with the CPower scoreboard
#[derive(Debug)]
pub struct ScoreboardClient {
    address: String,
    card_id: u8,
    stream: Option<TcpStream>,
}

impl ScoreboardClient {
    /// Create a new client
    pub fn new(address: String, card_id: u8) -> Self {
        Self {
            address,
            card_id,
            stream: None,
        }
    }

    /// Connect to the scoreboard
    pub async fn connect(&mut self) -> Result<()> {
        info!("Connecting to scoreboard at {}", self.address);
        
        let stream = timeout(Duration::from_secs(10), TcpStream::connect(&self.address))
            .await
            .context("Connection timeout")?
            .context("Failed to connect to scoreboard")?;
            
        self.stream = Some(stream);
        info!("Connected to scoreboard successfully");
        Ok(())
    }

    /// Disconnect from the scoreboard
    pub async fn disconnect(&mut self) {
        if let Some(mut stream) = self.stream.take() {
            let _ = stream.shutdown().await;
            info!("Disconnected from scoreboard");
        }
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    /// Send a command to the scoreboard
    pub async fn send_command(&mut self, command: Command) -> Result<Option<Vec<u8>>> {
        if self.stream.is_none() {
            self.connect().await?;
        }

        let packet = EthernetPacket::new(self.card_id, command.encode());
        let data = packet.encode();
        
        debug!("Sending packet: {:?}", packet);
        debug!("Raw bytes: {:02x?}", data);

        let stream = self.stream.as_mut().unwrap();
        
        stream.write_all(&data).await
            .context("Failed to send command")?;

        // Try to read response with timeout
        match timeout(Duration::from_secs(5), self.read_response()).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(e)) => {
                warn!("Error reading response: {}", e);
                Ok(None)
            }
            Err(_) => {
                warn!("Timeout reading response");
                Ok(None)
            }
        }
    }

    /// Read response from scoreboard
    async fn read_response(&mut self) -> Result<Option<Vec<u8>>> {
        if let Some(stream) = &mut self.stream {
            let mut buffer = vec![0u8; 1024];
            let n = stream.read(&mut buffer).await
                .context("Failed to read response")?;
            
            if n > 0 {
                buffer.truncate(n);
                debug!("Received {} bytes: {:02x?}", n, buffer);
                
                // Try to decode the packet
                match EthernetPacket::decode(&buffer) {
                    Ok(packet) => {
                        debug!("Decoded response packet: {:?}", packet);
                        Ok(Some(packet.command_data))
                    }
                    Err(e) => {
                        warn!("Failed to decode response packet: {}", e);
                        Ok(Some(buffer))
                    }
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Ensure connection is alive and reconnect if needed
    pub async fn ensure_connection(&mut self) -> Result<()> {
        if !self.is_connected() {
            info!("Connection lost, attempting to reconnect...");
            self.connect().await?;
        }
        Ok(())
    }

    /// Send keep-alive or test command
    pub async fn send_keep_alive(&mut self) -> Result<bool> {
        match self.send_command(Command::QueryVersion).await {
            Ok(_) => Ok(true),
            Err(e) => {
                error!("Keep-alive failed: {}", e);
                self.stream = None; // Mark as disconnected
                Ok(false)
            }
        }
    }
}

/// Connection manager to handle automatic reconnection
pub struct ConnectionManager {
    client: ScoreboardClient,
    reconnect_interval: Duration,
    keep_alive_interval: Duration,
}

impl ConnectionManager {
    pub fn new(address: String, card_id: u8) -> Self {
        Self {
            client: ScoreboardClient::new(address, card_id),
            reconnect_interval: Duration::from_secs(10),
            keep_alive_interval: Duration::from_secs(30),
        }
    }

    /// Start the connection manager
    pub async fn start(&mut self) -> Result<()> {
        self.client.connect().await?;
        
        let mut keep_alive_timer = tokio::time::interval(self.keep_alive_interval);
        
        loop {
            tokio::select! {
                _ = keep_alive_timer.tick() => {
                    if !self.client.send_keep_alive().await.unwrap_or(false) {
                        warn!("Keep-alive failed, attempting reconnection...");
                        self.attempt_reconnect().await;
                    }
                }
            }
        }
    }

    /// Attempt to reconnect with exponential backoff
    async fn attempt_reconnect(&mut self) {
        let mut delay = Duration::from_secs(1);
        let max_delay = Duration::from_secs(60);
        
        loop {
            match self.client.connect().await {
                Ok(_) => {
                    info!("Reconnected successfully");
                    return;
                }
                Err(e) => {
                    error!("Reconnection failed: {}", e);
                    sleep(delay).await;
                    delay = std::cmp::min(delay * 2, max_delay);
                }
            }
        }
    }

    /// Get a reference to the client
    pub fn client(&mut self) -> &mut ScoreboardClient {
        &mut self.client
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = ScoreboardClient::new("127.0.0.1:5200".to_string(), 0x01);
        assert!(!client.is_connected());
        assert_eq!(client.address, "127.0.0.1:5200");
        assert_eq!(client.card_id, 0x01);
    }
}
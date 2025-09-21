# HRUFC Scoreboard Control System

A server-based control system for the HRUFC scoreboard, built in Rust with a web frontend. This system replaces the proprietary software and provides reliable communication with the CPower scoreboard hardware via TCP/Ethernet.

![Scoreboard Web Interface](https://github.com/user-attachments/assets/b696094b-c8ba-4af7-b5c9-b2ce6845d75f)

## Features

### Core Functionality
- **Web-based Control Interface**: Modern HTML/JavaScript frontend for scoreboard control
- **Real-time Score Management**: Set and increment home/away scores with instant updates
- **Timer Control**: Start, stop, reset, and set match timers
- **Team Name Management**: Configure team names dynamically
- **Connection Resilience**: Automatic reconnection with exponential backoff
- **Status Monitoring**: Real-time connection status and error reporting

### Technical Features
- **CPower Protocol Implementation**: Full support for Ethernet communication protocol
- **Robust Error Handling**: Graceful degradation when scoreboard is disconnected
- **RESTful API**: Clean API endpoints for integration with other systems
- **Configurable**: Environment variable configuration for different setups
- **Production Ready**: Systemd service configuration included

## Architecture

The system consists of three main components:

1. **Protocol Layer** (`src/protocol/`): Implements the CPower communication protocol
   - Packet encoding/decoding for Ethernet communication
   - TCP client with automatic reconnection
   - Command abstractions for scoreboard operations

2. **Scoreboard Controller** (`src/scoreboard/`): High-level scoreboard management
   - State management for scores, timer, and team names
   - Display update coordination
   - Connection management

3. **Web Server** (`src/web/`): HTTP API and static file serving
   - RESTful API endpoints for all scoreboard operations
   - Static file serving for the web interface
   - CORS support for cross-origin requests

## Installation

### Prerequisites
- Rust 1.89.0 or later
- Access to the scoreboard network (typically port 5200)

### Building from Source

```bash
# Clone the repository
git clone <repository-url>
cd scoreboard

# Build the application
chmod +x build.sh
./build.sh

# Or build manually
cargo build --release
```

### Running the Server

#### Development Mode
```bash
# Set environment variables (optional)
export SCOREBOARD_ADDRESS=192.168.1.100:5200
export CARD_ID=1
export WEB_PORT=3030
export RUST_LOG=info

# Run the server
cargo run
# Or use the built binary
./target/release/scoreboard-server
```

#### Production Deployment
```bash
# Copy files to production location
sudo mkdir -p /opt/scoreboard
sudo cp target/release/scoreboard-server /opt/scoreboard/
sudo cp -r static /opt/scoreboard/

# Install systemd service
sudo cp scoreboard.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable scoreboard
sudo systemctl start scoreboard

# Check status
sudo systemctl status scoreboard
```

## Configuration

The application can be configured using environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `SCOREBOARD_ADDRESS` | `192.168.1.100:5200` | IP address and port of the scoreboard |
| `CARD_ID` | `1` | CPower card ID (1-254) |
| `WEB_PORT` | `3030` | Port for the web server |
| `RUST_LOG` | `info` | Log level (error, warn, info, debug, trace) |

Create a `.env` file in the project root or set environment variables:

```bash
cp .env.example .env
# Edit .env with your configuration
```

## API Documentation

The server provides a RESTful API for programmatic control:

### Status
- `GET /api/status` - Get current scoreboard status

### Team Management
- `POST /api/teams` - Update team names
  ```json
  { "home_team": "Team A", "away_team": "Team B" }
  ```

### Score Management
- `POST /api/scores` - Set scores directly
  ```json
  { "home_score": 10, "away_score": 5 }
  ```
- `POST /api/scores/home/increment` - Increment home score by 1
- `POST /api/scores/away/increment` - Increment away score by 1
- `POST /api/scores/reset` - Reset both scores to 0

### Timer Control
- `POST /api/timer` - Set timer
  ```json
  { "minutes": 45, "seconds": 0 }
  ```
- `POST /api/timer/start` - Start the timer
- `POST /api/timer/stop` - Stop the timer
- `POST /api/timer/reset` - Reset timer to 00:00

All endpoints return JSON responses with the format:
```json
{
  "success": true,
  "data": "Operation completed",
  "error": null
}
```

## Web Interface

Access the web interface by navigating to `http://localhost:3030` (or your configured port).

The interface provides:
- **Live Scoreboard Display**: Visual representation of the actual scoreboard
- **Team Controls**: Set team names with immediate preview
- **Score Controls**: Direct score setting and increment buttons
- **Timer Controls**: Full timer management with visual feedback
- **Connection Status**: Real-time connection monitoring
- **Responsive Design**: Works on desktop and mobile devices

## Protocol Implementation

This implementation follows the CPower Communication Protocol specification:

### Supported Commands
- **0x47**: Time Control (query, set, start/stop)
- **0x7b**: Display Messages (window creation, text display)

### Packet Format
The system uses the Ethernet packet format:
- 4-byte ID code (0xFFFFFFFF)
- 2-byte network data length
- 2-byte reserved field
- 1-byte packet type (0x68 for commands)
- 1-byte card type (0x32)
- 1-byte card ID
- Variable command data

### Window Layout
The standard 224x32 pixel display is divided into:
- Window 0: Home team name (96x16)
- Window 1: Home score (32x16) 
- Window 2: Away team name (96x16)
- Window 3: Away score (32x16)
- Window 4: Timer display (96x32)

## Troubleshooting

### Common Issues

**Cannot connect to scoreboard**
- Verify the scoreboard IP address and port (default 5200)
- Check network connectivity and firewall settings
- Ensure the scoreboard is powered on and connected to the network
- Try pinging the scoreboard IP address

**Web interface not accessible**
- Check if the server is running: `sudo systemctl status scoreboard`
- Verify the web port is not blocked by firewall
- Check server logs: `sudo journalctl -u scoreboard -f`

**Commands not updating the display**
- Check connection status in the web interface
- Verify the card ID matches the scoreboard configuration
- Review server logs for protocol errors

### Debugging

Enable debug logging:
```bash
export RUST_LOG=debug
./target/release/scoreboard-server
```

For more detailed protocol debugging:
```bash
export RUST_LOG=trace
./target/release/scoreboard-server
```

## Development

### Running Tests
```bash
cargo test
```

### Code Structure
- `src/main.rs`: Application entry point and configuration
- `src/protocol/`: CPower protocol implementation
  - `packet.rs`: Packet encoding/decoding
  - `commands.rs`: Command definitions and encoding
  - `client.rs`: TCP client with reconnection logic
- `src/scoreboard/`: High-level scoreboard management
- `src/web/`: Web server and API endpoints
- `static/index.html`: Web interface

### Adding New Features
1. Implement protocol commands in `src/protocol/commands.rs`
2. Add controller methods in `src/scoreboard/mod.rs`
3. Create API endpoints in `src/web/mod.rs`
4. Update the web interface in `static/index.html`

## License

MIT License - see LICENSE file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## Support

For issues and questions:
- Check the troubleshooting section above
- Review server logs for error details
- Open an issue on the project repository
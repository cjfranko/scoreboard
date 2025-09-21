#!/bin/bash
# Build script for the scoreboard server

set -e

echo "Building HRUFC Scoreboard Server..."

# Build in release mode
cargo build --release

echo "Build complete!"
echo "Binary location: target/release/scoreboard-server"
echo ""
echo "To run the server:"
echo "  RUST_LOG=info ./target/release/scoreboard-server"
echo ""
echo "Or copy the binary to /opt/scoreboard/ and use the systemd service:"
echo "  sudo cp target/release/scoreboard-server /opt/scoreboard/"
echo "  sudo cp -r static /opt/scoreboard/"
echo "  sudo cp scoreboard.service /etc/systemd/system/"
echo "  sudo systemctl daemon-reload"
echo "  sudo systemctl enable scoreboard"
echo "  sudo systemctl start scoreboard"
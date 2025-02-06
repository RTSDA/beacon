#!/bin/bash

# Build the application
cargo build --release

# Create necessary directories
sudo mkdir -p /usr/local/bin
sudo mkdir -p /usr/share/applications
sudo mkdir -p /usr/share/icons/hicolor/{32x32,64x64,128x128,256x256}/apps

# Copy the binary
sudo cp target/release/beacon /usr/local/bin/

# Copy the desktop file
sudo cp beacon.desktop /usr/share/applications/

# Copy icons to the appropriate directories
sudo cp icons/icon_32.png /usr/share/icons/hicolor/32x32/apps/beacon.png
sudo cp icons/icon_64.png /usr/share/icons/hicolor/64x64/apps/beacon.png
sudo cp icons/icon_128.png /usr/share/icons/hicolor/128x128/apps/beacon.png
sudo cp icons/icon_256.png /usr/share/icons/hicolor/256x256/apps/beacon.png

# Update icon cache
sudo gtk-update-icon-cache -f /usr/share/icons/hicolor

# Set permissions
sudo chmod +x /usr/local/bin/beacon
sudo chmod 644 /usr/share/applications/beacon.desktop

echo "Installation complete. You can now launch Beacon from your application menu." 
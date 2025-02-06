# Beacon

A modern digital signage application for displaying church events, built with Rust and Iced.

## Features

- Real-time event display with automatic updates
- Smooth image loading and transitions
- Modern, clean interface design
- Automatic event filtering based on date/time
- Support for high-resolution displays
- Efficient memory management for images

## Requirements

- Rust 1.70 or higher
- A running Pocketbase instance with events collection

## Configuration

Create a `config.toml` file in the application directory with the following settings:

```toml
pocketbase_url = "http://your-pocketbase-url"
window_width = 1920
window_height = 1080
slide_interval_secs = 10
refresh_interval_mins = 5
```

## Building

```bash
cargo build --release
```

## Running

```bash
./target/release/beacon
```

## Development

The application is built using:
- Iced for the UI framework
- Tokio for async runtime
- Reqwest for HTTP requests
- Chrono for date/time handling

## License

MIT License 
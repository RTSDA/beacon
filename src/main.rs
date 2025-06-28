mod config;
mod pocketbase;

use crate::pocketbase::ApiEvent;
use iced::widget::{column, row, image, container, text};
use iced::{
    window, Element,
    Length, Settings, Subscription, Theme, Task,
};
use iced::executor;
pub use iced::Program as IcedProgram;
use iced::Color;
use once_cell::sync::Lazy;
use std::time::Instant;
use iced::window::settings::PlatformSpecific;




static SETTINGS: Lazy<config::Settings> = Lazy::new(|| {
    config::Settings::new().unwrap_or_else(|e| {
        eprintln!("Failed to load config, using defaults: {}", e);
        config::Settings::default()
    })
});

static API_CLIENT: Lazy<pocketbase::ApiClient> = Lazy::new(|| {
    pocketbase::ApiClient::new(SETTINGS.api_url.clone())
});

// Define some constants for styling
const BACKGROUND_COLOR: Color = Color::from_rgb(0.05, 0.05, 0.08); // Slightly blue-tinted dark background
const ACCENT_COLOR: Color = Color::from_rgb(0.45, 0.27, 0.85); // Vibrant purple
const TEXT_COLOR: Color = Color::from_rgb(0.98, 0.98, 1.0);
const SECONDARY_TEXT_COLOR: Color = Color::from_rgb(0.85, 0.85, 0.95);
const CATEGORY_COLOR: Color = Color::from_rgb(0.45, 0.27, 0.85); // Match accent color
const DESCRIPTION_BG_COLOR: Color = Color::from_rgb(0.1, 0.1, 0.15); // Slightly blue-tinted
const TITLE_COLOR: Color = Color::from_rgb(1.0, 1.0, 0.95); // Warm white
const DATE_COLOR: Color = Color::from_rgb(0.95, 0.85, 1.0); // Light purple tint
const TIME_COLOR: Color = Color::from_rgb(0.8, 0.8, 0.95); // Soft purple-grey
const LOCATION_ICON_COLOR: Color = Color::from_rgb(0.6, 0.4, 0.9); // Brighter purple
const IMAGE_BG_COLOR: Color = Color::from_rgb(0.08, 0.08, 0.12); // Slightly lighter than background
const LOADING_FRAMES: [&str; 4] = ["⠋", "⠙", "⠹", "⠸"];
const MAX_IMAGE_SIZE: u64 = 2 * 1024 * 1024; // 2MB limit

#[derive(Debug)]
struct DigitalSign {
    events: Vec<Event>,
    current_event_index: usize,
    last_update: Instant,
    last_refresh: Instant,
    loaded_images: std::collections::HashMap<String, image::Handle>,
    loading_frame: usize,
    is_fetching: bool,
}

#[derive(Debug, Clone)]
struct Event {
    title: String,
    description: String,
    start_time: String,
    end_time: String,
    date: String,
    location: String,
    //location_url: Option<String>,
    image_url: Option<String>,
    category: String,
    //is_featured: bool,
    timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
    EventsLoaded(Vec<Event>),
    Error(String),
    ImageLoaded(String, image::Handle),
}

impl IcedProgram for DigitalSign {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type State = Self;
    type Renderer = iced::Renderer;

    fn title(&self, _state: &Self::State, _window_id: window::Id) -> String {
        String::from("Beacon")
    }

    fn update(&self, state: &mut Self::State, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                let mut tasks = vec![];
                state.loading_frame = (state.loading_frame + 1) % LOADING_FRAMES.len();

                if state.should_refresh() && !state.is_fetching {
                    tracing::info!("Refresh needed, starting event fetch");
                    state.is_fetching = true;
                    tasks.push(Task::perform(fetch_events(), Message::handle_result));
                }

                if !state.events.is_empty() 
                    && Instant::now().duration_since(state.last_update) >= SETTINGS.slide_interval() 
                {
                    let next_index = (state.current_event_index + 1) % state.events.len();
                    tracing::info!("Updating current event index from {} to {}", 
                        state.current_event_index,
                        next_index
                    );

                    // Clear all images that aren't needed anymore
                    let mut urls_to_remove = Vec::new();
                    for url in state.loaded_images.keys() {
                        let is_needed = state.events.iter().any(|e| {
                            e.image_url.as_ref().map_or(false, |event_url| event_url == url)
                        });
                        if !is_needed {
                            urls_to_remove.push(url.clone());
                        }
                    }
                    for url in urls_to_remove {
                        tracing::info!("Removing unused image: {}", url);
                        state.loaded_images.remove(&url);
                    }

                    // Update current index and load new image if needed
                    state.current_event_index = next_index;
                    state.last_update = Instant::now();

                    if let Some(current_event) = state.events.get(state.current_event_index) {
                        if let Some(url) = &current_event.image_url {
                            let url_clone = url.clone();
                            if !state.loaded_images.contains_key(&url_clone) {
                                tracing::info!("Starting image load for new current event: {}", url_clone);
                                let url_for_closure = url_clone.clone();
                                tasks.push(Task::perform(
                                    load_image(url_clone),
                                    move |handle| Message::ImageLoaded(url_for_closure.clone(), handle)
                                ));
                            } else {
                                tracing::info!("Image already loaded for current event: {}", url_clone);
                            }
                        }
                    }
                }

                if tasks.is_empty() {
                    Task::none()
                } else {
                    tracing::info!("Dispatching {} tasks", tasks.len());
                    Task::batch(tasks)
                }
            }
            Message::EventsLoaded(events) => {
                tracing::info!("Events loaded: {} events", events.len());
                
                // Clear all existing images as we have a new set of events
                state.loaded_images.clear();
                tracing::info!("Cleared all existing images");
                
                state.events = events;
                
                // Reset current event index if needed
                if state.current_event_index >= state.events.len() && !state.events.is_empty() {
                    tracing::info!("Resetting current event index from {} to 0", state.current_event_index);
                    state.current_event_index = 0;
                }
                
                state.last_refresh = Instant::now();
                state.is_fetching = false;
                
                // Load all images in parallel
                let mut image_tasks = Vec::new();
                
                // First, add the current event's image if it exists
                if let Some(event) = state.events.get(state.current_event_index) {
                    if let Some(url) = &event.image_url {
                        tracing::info!("Starting immediate load for current image: {}", url);
                        let url_clone = url.clone();
                        image_tasks.push(Task::perform(
                            load_image(url_clone.clone()),
                            move |handle| Message::ImageLoaded(url_clone.clone(), handle)
                        ));
                    }
                }
                
                // Then queue the rest of the images
                for (index, event) in state.events.iter().enumerate() {
                    if index != state.current_event_index {
                        if let Some(url) = &event.image_url {
                            tracing::info!("Queueing image preload for: {}", url);
                            let url_clone = url.clone();
                            image_tasks.push(Task::perform(
                                load_image(url_clone.clone()),
                                move |handle| Message::ImageLoaded(url_clone.clone(), handle)
                            ));
                        }
                    }
                }

                if !image_tasks.is_empty() {
                    tracing::info!("Starting load of {} images", image_tasks.len());
                    Task::batch(image_tasks)
                } else {
                    Task::none()
                }
            }
            Message::ImageLoaded(url, handle) => {
                tracing::info!("Image loaded: {}", url);
                state.loaded_images.insert(url, handle);
                Task::none()
            }
            Message::Error(error) => {
                tracing::error!("Error: {}", error);
                state.is_fetching = false;
                Task::none()
            }
        }
    }

    fn view<'a>(
        &self,
        state: &'a Self::State,
        _window_id: window::Id,
    ) -> Element<'a, Message, Theme, Self::Renderer> {
        let content: Element<'a, Message, Theme, Self::Renderer> = if let Some(event) = state.events.get(state.current_event_index) {
            let mut main_column = column![].spacing(40).padding(60).width(Length::Fill);

            // Left column with title and image
            let left_column = column![
                // Title with dynamic size and enhanced color
                container(
                    text(&event.title)
                        .size(if event.title.len() > 50 { 72 } else { 88 })
                        .style(|_: &Theme| text::Style { color: Some(TITLE_COLOR), ..Default::default() })
                )
                .width(Length::Fill)
                .padding(20),

                // Image container with enhanced styling
                container(
                    if let Some(ref image_url) = event.image_url {
                        if let Some(handle) = state.loaded_images.get(image_url) {
                            container(
                                image::Image::new(handle.clone())
                                    .width(Length::Fixed(900.0))
                                    .height(Length::Fixed(600.0))
                            )
                            .style(|_: &Theme| container::Style {
                                background: Some(IMAGE_BG_COLOR.into()),
                                ..Default::default()
                            })
                        } else {
                            container(
                                column![
                                    text(LOADING_FRAMES[state.loading_frame])
                                        .size(80)
                                        .style(|_: &Theme| text::Style { color: Some(ACCENT_COLOR), ..Default::default() }),
                                    text("Loading image...")
                                        .size(40)
                                        .style(|_: &Theme| text::Style { color: Some(SECONDARY_TEXT_COLOR), ..Default::default() })
                                ]
                                .spacing(20)
                                .align_x(iced::alignment::Horizontal::Center)
                            )
                        }
                    } else {
                        container(
                            text("No image available")
                                .size(32)
                                .style(|_: &Theme| text::Style { color: Some(SECONDARY_TEXT_COLOR), ..Default::default() })
                        )
                    }
                )
                .width(Length::Fixed(900.0))
                .height(Length::Fixed(600.0))
                .style(|_: &Theme| container::Style {
                    background: Some(IMAGE_BG_COLOR.into()),
                    ..Default::default()
                })
            ]
            .spacing(20);

            // Right column with category, date/time, location, and description
            let right_column = column![
                // Category badge with gradient-like effect
                container(
                    text(event.category.to_uppercase())
                        .size(36)
                        .style(|_: &Theme| text::Style { color: Some(TEXT_COLOR), ..Default::default() })
                )
                .padding(12)
                .style(|_: &Theme| container::Style {
                    background: Some(CATEGORY_COLOR.into()),
                    ..Default::default()
                }),

                // Date and time with enhanced colors
                container(
                    column![
                        text(&event.date)
                            .size(64)
                            .style(|_: &Theme| text::Style { color: Some(DATE_COLOR), ..Default::default() }),
                        text(format!("{} - {}", event.start_time, event.end_time))
                            .size(56)
                            .style(|_: &Theme| text::Style { color: Some(TIME_COLOR), ..Default::default() })
                    ]
                    .spacing(15)
                )
                .padding(20),

                // Location with colored icon
                if !event.location.is_empty() {
                    container(
                        row![
                            text("⌾")  // Using a more compatible location/target symbol
                                .size(48)
                                .style(|_: &Theme| text::Style { color: Some(LOCATION_ICON_COLOR), ..Default::default() }),
                            text(&event.location)
                                .size(48)
                                .style(|_: &Theme| text::Style { color: Some(SECONDARY_TEXT_COLOR), ..Default::default() })
                        ]
                        .spacing(15)
                        .align_y(iced::Alignment::Center)
                    )
                    .padding(20)
                } else {
                    container(text(""))
                },

                // Description with styled background
                container(
                    text(&event.description)
                        .size(44)
                        .style(|_: &Theme| text::Style { color: Some(TEXT_COLOR), ..Default::default() })
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(25)
                .style(|_: &Theme| container::Style {
                    background: Some(DESCRIPTION_BG_COLOR.into()),
                    ..Default::default()
                })
            ]
            .spacing(30)
            .width(Length::Fill)
            .height(Length::Fill);

            // Main content row
            let content_row = row![
                left_column,
                right_column
            ]
            .spacing(60)
            .height(Length::Fill);

            main_column = main_column.push(content_row);

            container(main_column)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_y(Length::Fill)
                .into()
        } else {
            container(
                text("Loading events...")
                    .size(64)
                    .style(|_: &Theme| text::Style { color: Some(ACCENT_COLOR), ..Default::default() })
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_: &Theme| container::Style {
                background: Some(BACKGROUND_COLOR.into()),
                ..Default::default()
            })
            .into()
    }

    fn subscription(&self, _state: &Self::State) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(100))
            .map(|_| Message::Tick)
    }

    fn theme(&self, _state: &Self::State, _window_id: window::Id) -> Theme {
        Theme::Dark
    }
}

impl Message {
    fn handle_result(result: Result<Vec<Event>, anyhow::Error>) -> Self {
        match result {
            Ok(events) => Message::EventsLoaded(events),
            Err(e) => Message::Error(e.to_string()),
        }
    }
}

async fn fetch_events() -> Result<Vec<Event>, anyhow::Error> {
    tracing::info!("Starting to fetch upcoming events from API");
    let api_events = match API_CLIENT.fetch_events().await {
        Ok(events) => {
            tracing::info!("Successfully fetched {} upcoming events from API", events.len());
            events
        },
        Err(e) => {
            tracing::error!("Failed to fetch events from API: {}", e);
            return Err(e);
        }
    };
    
    // Convert API events to display events (no filtering needed since /upcoming endpoint handles it)
    let mut events: Vec<Event> = api_events
        .into_iter()
        .map(Event::from)
        .collect();

    if events.is_empty() {
        tracing::warn!("No upcoming events found");
    } else {
        tracing::info!(
            "Found {} upcoming events, from {} to {}", 
            events.len(),
            events.first().map(|e| e.date.as_str()).unwrap_or("unknown"),
            events.last().map(|e| e.date.as_str()).unwrap_or("unknown")
        );
    }

    // Sort by start time (API should already provide them sorted, but ensure consistency)
    events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    tracing::info!("Processed {} upcoming events", events.len());
    Ok(events)
}

async fn load_image(url: String) -> image::Handle {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("Failed to create HTTP client");

    // First check the content length
    let head_resp = match client.head(&url).send().await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!("Failed to fetch image head {}: {}", url, e);
            return image::Handle::from_bytes(vec![]);
        }
    };

    if let Some(content_length) = head_resp.content_length() {
        tracing::info!("Image size for {}: {} KB", url, content_length / 1024);
        if content_length > MAX_IMAGE_SIZE {
            tracing::warn!("Image too large ({}KB), skipping download", content_length / 1024);
            return image::Handle::from_bytes(vec![]);
        }
    }

    let response = match client.get(&url).send().await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!("Failed to fetch image {}: {}", url, e);
            return image::Handle::from_bytes(vec![]);
        }
    };

    match response.bytes().await {
        Ok(bytes) => {
            if bytes.len() as u64 > MAX_IMAGE_SIZE {
                tracing::warn!("Image too large after download ({}KB), skipping", bytes.len() / 1024);
                return image::Handle::from_bytes(vec![]);
            }
            tracing::info!("Successfully downloaded image {} with {} bytes", url, bytes.len());
            image::Handle::from_bytes(bytes.to_vec())
        }
        Err(e) => {
            tracing::error!("Failed to get image bytes for {}: {}", url, e);
            image::Handle::from_bytes(vec![])
        }
    }
}

impl From<ApiEvent> for Event {
    fn from(event: ApiEvent) -> Self {
        let clean_description = html2text::from_read(event.description.as_bytes(), 80)
            .replace('\n', " ")
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ");

        let date = event.start_time.format("%A, %B %d, %Y").to_string();
        let start_time = event.start_time.format("%I:%M %p").to_string().trim_start_matches('0').to_string();
        let end_time = event.end_time.format("%I:%M %p").to_string().trim_start_matches('0').to_string();

        let image_url = event.image.clone();
        if let Some(ref url) = image_url {
            tracing::info!("Using image URL: {}", url);
        }

        Self {
            title: event.title,
            description: clean_description,
            start_time,
            end_time,
            date,
            location: event.location,
            //location_url: event.location_url,
            image_url,
            category: event.category,
            //is_featured: event.is_featured,
            timestamp: event.start_time,
        }
    }
}

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("Starting Beacon Digital Signage");
    tracing::info!("API URL: {}", SETTINGS.api_url);

    // Load the icon file
    let icon_data = {
        // Try local development path first
        let local_path = "icons/appicon.png";
        // Try system-wide installation path
        let system_paths = [
            "/usr/share/icons/hicolor/256x256/apps/beacon.png",
            "/usr/local/share/icons/hicolor/256x256/apps/beacon.png",
        ];
        
        let mut icon_bytes = None;
        
        // Try local path first
        if let Ok(bytes) = std::fs::read(local_path) {
            tracing::info!("Found icon in local path: {}", local_path);
            icon_bytes = Some(bytes);
        } else {
            // Try system paths
            for path in system_paths.iter() {
                if let Ok(bytes) = std::fs::read(path) {
                    tracing::info!("Found icon in system path: {}", path);
                    icon_bytes = Some(bytes);
                    break;
                }
            }
        }
        
        // Create icon from bytes if we found any
        if let Some(bytes) = icon_bytes {
            match window::icon::from_file_data(&bytes, None) {
                Ok(icon) => {
                    tracing::info!("Successfully created icon from data");
                    Some(icon)
                }
                Err(e) => {
                    tracing::error!("Failed to create icon from data: {}", e);
                    None
                }
            }
        } else {
            tracing::error!("Could not find icon file in any location");
            None
        }
    };

    let window_settings = window::Settings {
        size: iced::Size::new(SETTINGS.window_width as f32, SETTINGS.window_height as f32),
        position: window::Position::Centered,
        fullscreen: true,
        resizable: false,
        decorations: false,
        icon: icon_data,
        #[cfg(target_os = "macos")]
        platform_specific: PlatformSpecific {
            title_hidden: true,
            titlebar_transparent: true,
            fullsize_content_view: true,
        },
        #[cfg(not(target_os = "macos"))]
        platform_specific: PlatformSpecific::default(),
        ..Default::default()
    };

    let settings = Settings {
       // window: window_settings,
        //flags: (),
        fonts: vec![],
        default_font: iced::Font::default(),
        antialiasing: true,
        ..Default::default()
    };

    // Create the initial state and start loading events
    let mut app = DigitalSign::default();
    app.is_fetching = true;

    DigitalSign::run_with(
        app,
        settings,
        Some(window_settings),
        || {
            let mut state = DigitalSign::default();
            state.is_fetching = true;
            (
                state,
                Task::perform(
                    fetch_events(),
                    Message::handle_result
                )
            )
        }
    )
}

impl DigitalSign {
    fn should_refresh(&self) -> bool {
        let elapsed = self.last_refresh.elapsed();
        let interval = SETTINGS.refresh_interval();
        let should_refresh = elapsed >= interval;
        tracing::info!(
            "Checking refresh: elapsed={:?}, interval={:?}, should_refresh={}",
            elapsed,
            interval,
            should_refresh
        );
        should_refresh
    }
}

impl Default for DigitalSign {
    fn default() -> Self {
        Self {
            events: vec![],
            current_event_index: 0,
            last_update: Instant::now(),
            last_refresh: Instant::now(),
            loaded_images: std::collections::HashMap::new(),
            loading_frame: 0,
            is_fetching: false,
        }
    }
}

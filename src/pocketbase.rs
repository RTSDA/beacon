use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const POCKETBASE_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Serialize, Deserialize)]
pub struct PocketbaseEvent {
    pub id: String,
    pub title: String,
    pub description: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub location: String,
    pub location_url: Option<String>,
    pub image: Option<String>,
    pub thumbnail: Option<String>,
    pub category: String,
    pub is_featured: bool,
    pub reoccuring: String,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PocketbaseClient {
    client: reqwest::Client,
    base_url: String,
}

impl PocketbaseClient {
    pub fn new(base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(POCKETBASE_TIMEOUT)
            .build()
            .expect("Failed to create HTTP client");

        Self { client, base_url }
    }

    pub async fn fetch_events(&self) -> Result<Vec<PocketbaseEvent>> {
        // Subtract 12 hours from now to include upcoming events
        let now = (chrono::Utc::now() - chrono::Duration::hours(12)).to_rfc3339();
        let url = format!(
            "{}/api/collections/events/records?filter=(end_time>='{}')",
            self.base_url,
            now
        );
        tracing::info!("Fetching events from URL: {}", url);
        
        let response = match self.client.get(&url)
            .header("Cache-Control", "max-age=60") // Cache for 60 seconds
            .send()
            .await 
        {
            Ok(resp) => {
                tracing::info!("Got response with status: {}", resp.status());
                resp
            },
            Err(e) => {
                tracing::error!("HTTP request failed: {}", e);
                return Err(e.into());
            }
        };

        let response = match response.error_for_status() {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!("HTTP error status: {}", e);
                return Err(e.into());
            }
        };

        #[derive(Deserialize)]
        struct Response {
            items: Vec<PocketbaseEvent>,
        }

        match response.json().await {
            Ok(data) => {
                let Response { items } = data;
                tracing::info!("Successfully parsed {} events from response", items.len());
                Ok(items)
            },
            Err(e) => {
                tracing::error!("Failed to parse JSON response: {}", e);
                Err(e.into())
            }
        }
    }
} 
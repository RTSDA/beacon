use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const API_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiEvent {
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
    pub recurring_type: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ApiClient {
    client: reqwest::Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(API_TIMEOUT)
            .build()
            .expect("Failed to create HTTP client");

        Self { client, base_url }
    }

    pub async fn fetch_events(&self) -> Result<Vec<ApiEvent>> {
        let url = format!("{}/api/events/upcoming", self.base_url);
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
        struct ApiResponse {
            success: bool,
            data: Vec<ApiEvent>,
        }

        match response.json().await {
            Ok(api_response) => {
                let ApiResponse { success, data } = api_response;
                if success {
                    tracing::info!("Successfully parsed {} events from response", data.len());
                    Ok(data)
                } else {
                    tracing::error!("API returned success: false");
                    Err(anyhow::anyhow!("API request failed"))
                }
            },
            Err(e) => {
                tracing::error!("Failed to parse JSON response: {}", e);
                Err(e.into())
            }
        }
    }
} 
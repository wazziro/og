use chrono::{Local, NaiveDate, NaiveTime, TimeZone, Utc};
use google_calendar3::{CalendarHub, hyper, hyper_rustls};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use yup_oauth2::{
    ApplicationSecret, InstalledFlowAuthenticator, InstalledFlowReturnMethod,
};

#[derive(Debug, Clone)]
pub struct CalendarEvent {
    pub start_time: Option<NaiveTime>,
    pub end_time: Option<NaiveTime>,
    pub title: String,
    pub is_all_day: bool,
}

impl CalendarEvent {
    pub fn format_with_time(&self) -> String {
        if self.is_all_day {
            format!("00:00-23:59 {}", self.title)
        } else if let (Some(start), Some(end)) = (self.start_time, self.end_time) {
            format!("{}-{} {}", 
                start.format("%H:%M"),
                end.format("%H:%M"),
                self.title
            )
        } else {
            format!("00:00-23:59 {}", self.title)
        }
    }

    pub fn format_title_only(&self) -> String {
        self.title.clone()
    }
}

#[derive(Deserialize)]
struct Credentials {
    installed: InstalledCredentials,
}

#[derive(Deserialize)]
struct InstalledCredentials {
    client_id: String,
    client_secret: String,
    auth_uri: String,
    token_uri: String,
    redirect_uris: Vec<String>,
}

pub async fn get_today_events(_show_title_only: bool) -> Result<Vec<CalendarEvent>, Box<dyn Error>> {
    let hub = create_calendar_hub().await?;
    let today = Local::now().date_naive();
    let events = fetch_events_for_date(&hub, today).await?;
    Ok(events)
}

async fn create_calendar_hub() -> Result<CalendarHub<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>, Box<dyn Error>> {
    let credentials_path = get_credentials_path()?;
    let token_path = get_token_path()?;
    
    // Read credentials manually
    let credentials_content = fs::read_to_string(&credentials_path)
        .map_err(|e| format!("Failed to read credentials file: {}", e))?;
    
    
    let credentials: Credentials = serde_json::from_str(&credentials_content)
        .map_err(|e| format!("Failed to parse credentials file: {}", e))?;
    
    // Create application secret manually
    let app_secret = ApplicationSecret {
        client_id: credentials.installed.client_id,
        client_secret: credentials.installed.client_secret,
        auth_uri: credentials.installed.auth_uri,
        token_uri: credentials.installed.token_uri,
        redirect_uris: credentials.installed.redirect_uris,
        ..Default::default()
    };
    
    // Create authenticator
    let auth = InstalledFlowAuthenticator::builder(
        app_secret,
        InstalledFlowReturnMethod::HTTPRedirect
    )
    .persist_tokens_to_disk(&token_path)
    .build()
    .await
    .map_err(|e| {
        let error_msg = format!("{}", e);
        if error_msg.contains("access_denied") || error_msg.contains("unauthorized") {
            format!("Google OAuth access denied. This application may not be verified by Google. You need to:\n1. Create your own Google Cloud project\n2. Enable Calendar API\n3. Create OAuth credentials\n4. Replace the credentials.json file")
        } else {
            format!("Authentication failed: {}", e)
        }
    })?;
    
    // Create HTTPS connector with proper configuration for hyper-rustls 0.25
    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()?
        .https_or_http()
        .enable_http1()
        .build();
    
    let client = hyper::Client::builder().build(https);
    let hub = CalendarHub::new(client, auth);
    Ok(hub)
}

async fn fetch_events_for_date(
    hub: &CalendarHub<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>,
    date: NaiveDate
) -> Result<Vec<CalendarEvent>, Box<dyn Error>> {
    let start_time = date.and_hms_opt(0, 0, 0).unwrap();
    let end_time = date.and_hms_opt(23, 59, 59).unwrap();
    
    // Convert to UTC for API call
    let start_utc = Local.from_local_datetime(&start_time).unwrap().with_timezone(&Utc);
    let end_utc = Local.from_local_datetime(&end_time).unwrap().with_timezone(&Utc);
    
    let result = hub.events()
        .list("primary")
        .time_min(start_utc)
        .time_max(end_utc)
        .single_events(true)
        .order_by("startTime")
        .doit()
        .await;
    
    match result {
        Ok((_, events_list)) => {
            let mut calendar_events = Vec::new();
            
            if let Some(items) = events_list.items {
                for event in items {
                    let title = event.summary.unwrap_or_else(|| "No Title".to_string());
                    
                    let (start_time, end_time, is_all_day) = if let Some(start) = event.start {
                        if let Some(date_time) = start.date_time {
                            // Timed event
                            let start_local = date_time.with_timezone(&Local);
                            let start_naive = start_local.time();
                            
                            let end_naive = if let Some(end) = event.end {
                                if let Some(end_date_time) = end.date_time {
                                    let end_local = end_date_time.with_timezone(&Local);
                                    end_local.time()
                                } else {
                                    start_naive
                                }
                            } else {
                                start_naive
                            };
                            
                            (Some(start_naive), Some(end_naive), false)
                        } else {
                            // All-day event
                            (None, None, true)
                        }
                    } else {
                        (None, None, true)
                    };
                    
                    calendar_events.push(CalendarEvent {
                        start_time,
                        end_time,
                        title,
                        is_all_day,
                    });
                }
            }
            
            Ok(calendar_events)
        }
        Err(e) => {
            let error_msg = format!("{}", e);
            if error_msg.contains("access_denied") || error_msg.contains("unauthorized") {
                Err("Google Calendar access denied. The OAuth application may not be verified. Please check with the app developer or use your own Google Cloud credentials.".into())
            } else {
                Err(format!("Failed to fetch calendar events: {}", e).into())
            }
        }
    }
}

fn get_credentials_path() -> Result<PathBuf, Box<dyn Error>> {
    let home_dir = dirs::home_dir()
        .ok_or("Could not determine home directory")?;
    
    let path = home_dir.join(".config").join("google").join("credentials.json");
    
    if !path.exists() {
        return Err(format!("Credentials file not found: {}", path.display()).into());
    }
    
    Ok(path)
}

fn get_token_path() -> Result<PathBuf, Box<dyn Error>> {
    let home_dir = dirs::home_dir()
        .ok_or("Could not determine home directory")?;
    
    let path = home_dir.join(".config").join("google").join("token.json");
    Ok(path)
}

pub fn format_events_output(events: &[CalendarEvent], show_title_only: bool) -> String {
    let mut output = String::from("### 予定\n");
    
    if events.is_empty() {
        output.push_str("予定はありません。\n");
    } else {
        for event in events {
            if show_title_only {
                output.push_str(&format!("{}\n", event.format_title_only()));
            } else {
                output.push_str(&format!("{}\n", event.format_with_time()));
            }
        }
    }
    
    output
}
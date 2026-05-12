use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use clap::Parser;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use std::io::Write;

/// iCloud Calendar Private API - Command Line Tool
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// iCloud username (Apple ID)
    #[arg(short, long)]
    username: String,

    /// iCloud password (or app-specific password)
    #[arg(short, long)]
    password: String,

    /// Output file for iCal data (default: stdout)
    #[arg(short, long)]
    output: Option<String>,

    /// Calendar name to export (default: all calendars)
    #[arg(short, long)]
    calendar: Option<String>,
}

struct ICloudCalendarClient {
    client: reqwest::Client,
    username: String,
    password: String,
    principal_url: Option<String>,
    calendar_home_url: Option<String>,
}

impl ICloudCalendarClient {
    fn new(username: String, password: String) -> Result<Self> {
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .build()?;

        Ok(Self {
            client,
            username,
            password,
            principal_url: None,
            calendar_home_url: None,
        })
    }

    fn get_auth_header(&self) -> String {
        let credentials = format!("{}:{}", self.username, self.password);
        let encoded = general_purpose::STANDARD.encode(credentials.as_bytes());
        format!("Basic {}", encoded)
    }

    fn build_url(&self, href: &str) -> String {
        // If href is already an absolute URL, use it as-is
        if href.starts_with("http://") || href.starts_with("https://") {
            href.to_string()
        } else {
            // Otherwise, prepend the base CalDAV URL
            format!("https://caldav.icloud.com{}", href)
        }
    }

    fn get_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&self.get_auth_header()).unwrap(),
        );
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/xml; charset=utf-8"),
        );
        headers.insert("Depth", HeaderValue::from_static("0"));
        headers
    }

    async fn discover_principal(&mut self) -> Result<()> {
        println!("🔍 Discovering CalDAV principal...");
        
        let caldav_url = "https://caldav.icloud.com/";
        let propfind_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<d:propfind xmlns:d="DAV:">
  <d:prop>
    <d:current-user-principal />
  </d:prop>
</d:propfind>"#;

        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"PROPFIND")?, caldav_url)
            .headers(self.get_headers())
            .body(propfind_body)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to discover principal: {} - {}",
                response.status(),
                response.text().await?
            );
        }

        let body = response.text().await?;
        
        // Debug output
        if std::env::var("DEBUG").is_ok() {
            println!("\n=== DEBUG: Principal Response ===");
            println!("{}", body);
            println!("=== END DEBUG ===\n");
        }
        
        // Parse the principal URL from the response - try multiple patterns
        let href = self.extract_href(&body, "current-user-principal")
            .context("Could not find principal URL in response")?;
        
        self.principal_url = Some(href.clone());
        println!("✅ Found principal: {}", href);

        Ok(())
    }

    async fn discover_calendar_home(&mut self) -> Result<()> {
        println!("🔍 Discovering calendar home...");
        
        let principal_url = self
            .principal_url
            .as_ref()
            .context("Principal URL not set")?;
        
        let full_url = self.build_url(principal_url);
        
        let propfind_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <c:calendar-home-set />
  </d:prop>
</d:propfind>"#;

        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"PROPFIND")?, &full_url)
            .headers(self.get_headers())
            .body(propfind_body)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to discover calendar home: {} - {}",
                response.status(),
                response.text().await?
            );
        }

        let body = response.text().await?;
        
        // Debug output
        if std::env::var("DEBUG").is_ok() {
            println!("\n=== DEBUG: Calendar Home Response ===");
            println!("{}", body);
            println!("=== END DEBUG ===\n");
        }
        
        // Parse the calendar home URL from the response
        let href = self.extract_href(&body, "calendar-home-set")
            .context("Could not find calendar home URL in response")?;
        
        self.calendar_home_url = Some(href.clone());
        println!("✅ Found calendar home: {}", href);

        Ok(())
    }

    async fn list_calendars(&self) -> Result<Vec<CalendarInfo>> {
        println!("📅 Listing calendars...");
        
        let calendar_home = self
            .calendar_home_url
            .as_ref()
            .context("Calendar home URL not set")?;
        
        let full_url = self.build_url(calendar_home);
        
        let mut headers = self.get_headers();
        headers.insert("Depth", HeaderValue::from_static("1"));
        
        let propfind_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" xmlns:cs="http://calendarserver.org/ns/" xmlns:a="http://apple.com/ns/ical/">
  <d:prop>
    <d:resourcetype />
    <d:displayname />
    <a:calendar-color />
    <c:calendar-description />
  </d:prop>
</d:propfind>"#;

        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"PROPFIND")?, &full_url)
            .headers(headers)
            .body(propfind_body)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to list calendars: {} - {}",
                response.status(),
                response.text().await?
            );
        }

        let body = response.text().await?;
        
        // Debug output
        if std::env::var("DEBUG").is_ok() {
            println!("\n=== DEBUG: Calendar List Response ===");
            println!("{}", body);
            println!("=== END DEBUG ===\n");
        }
        
        let calendars = self.parse_calendars(&body)?;
        
        println!("✅ Found {} calendar(s)", calendars.len());
        for cal in &calendars {
            println!("   - {} ({})", cal.display_name, cal.url);
        }

        Ok(calendars)
    }

    fn extract_href(&self, xml: &str, context: &str) -> Option<String> {
        // For principal, look specifically after current-user-principal
        // For calendar-home, look after calendar-home-set
        let search_start = if context == "current-user-principal" {
            xml.find("current-user-principal").unwrap_or(0)
        } else if context == "calendar-home-set" {
            xml.find("calendar-home-set").unwrap_or(0)
        } else {
            0
        };
        
        let xml_section = &xml[search_start..];
        
        // Try different href tag patterns (with namespace prefixes or attributes)
        let href_patterns = [
            "<d:href>",
            "<D:href>",
            "<href>",
            "<href ",  // href with attributes like <href xmlns="DAV:">
            "<HREF>",
            "<HREF ",
        ];
        
        for start_pattern in &href_patterns {
            if let Some(start) = xml_section.find(start_pattern) {
                // Find the end of the opening tag (could have attributes)
                let after_tag_name = start + start_pattern.len();
                let content_start = if start_pattern.ends_with(' ') {
                    // Tag has attributes, find the closing >
                    if let Some(close_bracket) = xml_section[after_tag_name..].find('>') {
                        after_tag_name + close_bracket + 1
                    } else {
                        continue;
                    }
                } else {
                    after_tag_name
                };
                
                // Now find the closing tag
                let end_patterns = ["</d:href>", "</D:href>", "</href>", "</HREF>"];
                for end_pattern in &end_patterns {
                    if let Some(end) = xml_section[content_start..].find(end_pattern) {
                        let href = xml_section[content_start..content_start + end].trim();
                        if !href.is_empty() {
                            return Some(href.to_string());
                        }
                    }
                }
            }
        }
        
        None
    }
    
    fn parse_calendars(&self, xml: &str) -> Result<Vec<CalendarInfo>> {
        let mut calendars = Vec::new();
        
        // Simple XML parsing - in production, use a proper XML parser
        let responses: Vec<&str> = xml.split("<response").collect();
        
        for response in responses.iter().skip(1) {
            // Check if this is a calendar resource
            // Look for <calendar> tag (with or without namespace prefix)
            if !response.contains("<calendar") && !response.contains("<c:calendar") {
                continue;
            }
            
            let mut url = String::new();
            let mut display_name = String::new();
            
            // Extract href using more robust parsing
            let href_patterns = [("<href>", "</href>"), ("<href ", "</href>"), ("<d:href>", "</d:href>"), ("<d:href ", "</d:href>")];
            for (start_pattern, end_pattern) in &href_patterns {
                if let Some(start) = response.find(start_pattern) {
                    let after_tag = start + start_pattern.len();
                    let content_start = if start_pattern.ends_with(' ') {
                        // Has attributes, find closing >
                        if let Some(close) = response[after_tag..].find('>') {
                            after_tag + close + 1
                        } else {
                            continue;
                        }
                    } else {
                        after_tag
                    };
                    
                    if let Some(end) = response[content_start..].find(end_pattern) {
                        url = response[content_start..content_start + end].trim().to_string();
                        break;
                    }
                }
            }
            
            // Extract display name - handle both with and without xmlns attribute
            let displayname_patterns = [("<displayname>", "</displayname>"), ("<displayname ", "</displayname>"), ("<d:displayname>", "</d:displayname>"), ("<d:displayname ", "</d:displayname>")];
            for (start_pattern, end_pattern) in &displayname_patterns {
                if let Some(start) = response.find(start_pattern) {
                    let after_tag = start + start_pattern.len();
                    let content_start = if start_pattern.ends_with(' ') {
                        // Has attributes, find closing >
                        if let Some(close) = response[after_tag..].find('>') {
                            after_tag + close + 1
                        } else {
                            continue;
                        }
                    } else {
                        after_tag
                    };
                    
                    if let Some(end) = response[content_start..].find(end_pattern) {
                        display_name = response[content_start..content_start + end].trim().to_string();
                        break;
                    }
                }
            }
            
            if !url.is_empty() && url.ends_with('/') {
                // Skip special system calendars and the calendar home itself
                let skip_paths = [
                    "/inbox/",
                    "/outbox/",
                    "/notification/",
                    "/calendars/",  // Calendar home itself
                ];
                
                let should_skip = skip_paths.iter().any(|&skip| url.ends_with(skip));
                
                if !should_skip {
                    calendars.push(CalendarInfo {
                        url,
                        display_name: if display_name.is_empty() {
                            "Unnamed Calendar".to_string()
                        } else {
                            display_name
                        },
                    });
                }
            }
        }
        
        Ok(calendars)
    }

    async fn get_calendar_events(&self, calendar_url: &str) -> Result<String> {
        println!("📥 Fetching events from: {}", calendar_url);
        
        let full_url = self.build_url(calendar_url);
        
        let mut headers = self.get_headers();
        headers.insert("Depth", HeaderValue::from_static("1"));
        
        // REPORT request to get all calendar objects
        let report_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <d:getetag />
    <c:calendar-data />
  </d:prop>
  <c:filter>
    <c:comp-filter name="VCALENDAR">
      <c:comp-filter name="VEVENT" />
    </c:comp-filter>
  </c:filter>
</c:calendar-query>"#;

        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"REPORT")?, &full_url)
            .headers(headers)
            .body(report_body)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to get calendar events: {} - {}",
                response.status(),
                response.text().await?
            );
        }

        let body = response.text().await?;
        
        // Debug output
        if std::env::var("DEBUG").is_ok() {
            println!("\n=== DEBUG: Events Response for {} ===", calendar_url);
            println!("{}", if body.len() > 2000 { 
                format!("{}... (truncated, {} bytes total)", &body[..2000], body.len())
            } else {
                body.clone()
            });
            println!("=== END DEBUG ===\n");
        }
        
        Ok(body)
    }

    fn extract_ical_events(&self, xml_response: &str) -> Result<Vec<String>> {
        let mut events = Vec::new();
        
        // Try multiple patterns for calendar-data elements
        let patterns = [
            "<calendar-data xmlns=",
            "<calendar-data>",
            "<c:calendar-data>",
            "<C:calendar-data>",
            "<cal:calendar-data>",
        ];
        
        for start_pattern in &patterns {
            let mut pos = 0;
            while let Some(start) = xml_response[pos..].find(start_pattern) {
                let after_tag_start = pos + start + start_pattern.len();
                
                // Find the end of the opening tag (handle attributes)
                let content_start = if start_pattern.contains(' ') || start_pattern.ends_with('=') {
                    // Has attributes like xmlns=, find the closing >
                    if let Some(close_bracket) = xml_response[after_tag_start..].find('>') {
                        after_tag_start + close_bracket + 1
                    } else {
                        pos = after_tag_start;
                        continue;
                    }
                } else {
                    after_tag_start
                };
                
                // Find the closing tag
                let end_patterns = ["</calendar-data>", "</c:calendar-data>", "</C:calendar-data>", "</cal:calendar-data>"];
                let mut found_end = None;
                for end_pattern in &end_patterns {
                    if let Some(end) = xml_response[content_start..].find(end_pattern) {
                        found_end = Some(content_start + end);
                        break;
                    }
                }
                
                if let Some(end_idx) = found_end {
                    let mut ical_data = xml_response[content_start..end_idx].to_string();
                    
                    // Handle CDATA wrapper
                    if ical_data.starts_with("<![CDATA[") {
                        ical_data = ical_data.strip_prefix("<![CDATA[").unwrap_or(&ical_data).to_string();
                        if let Some(cdata_end) = ical_data.rfind("]]>") {
                            ical_data = ical_data[..cdata_end].to_string();
                        }
                    }
                    
                    // Unescape XML entities (in case not using CDATA)
                    let unescaped = ical_data
                        .replace("&lt;", "<")
                        .replace("&gt;", ">")
                        .replace("&amp;", "&")
                        .replace("&quot;", "\"")
                        .replace("&#13;", "\r");
                    
                    // Only add if it looks like valid iCal data
                    if unescaped.contains("BEGIN:VCALENDAR") || unescaped.contains("BEGIN:VEVENT") {
                        events.push(unescaped);
                    }
                    
                    pos = end_idx;
                } else {
                    break;
                }
            }
            
            // If we found events with this pattern, stop trying other patterns
            if !events.is_empty() {
                break;
            }
        }
        
        if std::env::var("DEBUG").is_ok() && events.is_empty() {
            println!("⚠️  DEBUG: No calendar-data elements found in response");
            println!("   Response length: {} bytes", xml_response.len());
            println!("   Contains 'calendar-data': {}", xml_response.contains("calendar-data"));
            println!("   Contains 'VCALENDAR': {}", xml_response.contains("VCALENDAR"));
            println!("   Contains 'VEVENT': {}", xml_response.contains("VEVENT"));
            println!("   Contains 'CDATA': {}", xml_response.contains("CDATA"));
        }
        
        println!("✅ Extracted {} event(s)", events.len());
        Ok(events)
    }

    fn combine_ical_events(&self, events: Vec<String>, calendar_name: &str) -> String {
        let mut combined = String::new();
        
        // Create a wrapper VCALENDAR
        combined.push_str("BEGIN:VCALENDAR\r\n");
        combined.push_str("VERSION:2.0\r\n");
        combined.push_str("PRODID:-//iCloud Calendar Export//EN\r\n");
        combined.push_str("CALSCALE:GREGORIAN\r\n");
        combined.push_str(&format!("X-WR-CALNAME:{}\r\n", calendar_name));
        combined.push_str("X-WR-TIMEZONE:UTC\r\n");
        
        // Extract and add all VEVENT components
        for event in events {
            // Find all VEVENT components in this calendar data
            let mut pos = 0;
            while let Some(start) = event[pos..].find("BEGIN:VEVENT") {
                if let Some(end) = event[pos + start..].find("END:VEVENT") {
                    let vevent = &event[pos + start..pos + start + end + 10]; // +10 for "END:VEVENT"
                    combined.push_str(vevent);
                    combined.push_str("\r\n");
                    pos = pos + start + end + 10;
                } else {
                    break;
                }
            }
        }
        
        combined.push_str("END:VCALENDAR\r\n");
        combined
    }

    async fn export_calendar(&mut self, calendar_filter: Option<String>) -> Result<String> {
        // Discover the CalDAV endpoints
        self.discover_principal().await?;
        self.discover_calendar_home().await?;
        
        // Get list of calendars
        let calendars = self.list_calendars().await?;
        
        if calendars.is_empty() {
            anyhow::bail!("No calendars found");
        }
        
        // Filter calendars if requested
        let calendars_to_export: Vec<&CalendarInfo> = if let Some(filter) = calendar_filter {
            calendars
                .iter()
                .filter(|c| c.display_name.contains(&filter))
                .collect()
        } else {
            calendars.iter().collect()
        };
        
        if calendars_to_export.is_empty() {
            anyhow::bail!("No calendars match the filter");
        }
        
        let mut all_ical = String::new();
        let mut success_count = 0;
        let mut error_count = 0;
        
        for calendar in calendars_to_export {
            match self.get_calendar_events(&calendar.url).await {
                Ok(xml_response) => {
                    match self.extract_ical_events(&xml_response) {
                        Ok(events) => {
                            if !events.is_empty() {
                                let ical = self.combine_ical_events(events, &calendar.display_name);
                                all_ical.push_str(&ical);
                                all_ical.push_str("\r\n");
                                success_count += 1;
                            } else {
                                println!("   ℹ️  '{}' has no events", calendar.display_name);
                            }
                        }
                        Err(e) => {
                            eprintln!("   ⚠️  Failed to parse events from '{}': {}", calendar.display_name, e);
                            error_count += 1;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("   ⚠️  Failed to fetch '{}': {}", calendar.display_name, e);
                    error_count += 1;
                }
            }
        }
        
        if success_count == 0 {
            anyhow::bail!("No calendars could be exported successfully");
        }
        
        println!("\n✅ Successfully exported {} calendar(s)", success_count);
        if error_count > 0 {
            println!("⚠️  {} calendar(s) had errors and were skipped", error_count);
        }
        
        Ok(all_ical)
    }
}

#[derive(Debug)]
struct CalendarInfo {
    url: String,
    display_name: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("🍎 iCloud Calendar Private API - CLI Tool");
    println!("==========================================\n");
    println!("Connecting to iCloud as: {}", args.username);
    println!();

    let mut client = ICloudCalendarClient::new(args.username, args.password)?;
    
    let ical_data = client.export_calendar(args.calendar).await?;
    
    // Output the iCal data
    match args.output {
        Some(output_file) => {
            let mut file = std::fs::File::create(&output_file)?;
            file.write_all(ical_data.as_bytes())?;
            println!("\n✅ Calendar exported to: {}", output_file);
        }
        None => {
            println!("\n📄 iCal Output:");
            println!("================\n");
            println!("{}", ical_data);
        }
    }

    Ok(())
}

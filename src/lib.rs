use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

pub struct ICloudCalendarClient {
    client: reqwest::Client,
    username: String,
    password: String,
    principal_url: Option<String>,
    calendar_home_url: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CalendarInfo {
    pub url: String,
    pub display_name: String,
}

impl ICloudCalendarClient {
    pub fn new(username: String, password: String) -> Result<Self> {
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
        
        let href = self.extract_href(&body, "current-user-principal")
            .context("Could not find principal URL in response")?;
        
        self.principal_url = Some(href);
        Ok(())
    }

    async fn discover_calendar_home(&mut self) -> Result<()> {
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
        
        let href = self.extract_href(&body, "calendar-home-set")
            .context("Could not find calendar home URL in response")?;
        
        self.calendar_home_url = Some(href);
        Ok(())
    }

    pub async fn list_calendars(&mut self) -> Result<Vec<CalendarInfo>> {
        // Ensure discovery is done
        if self.principal_url.is_none() {
            self.discover_principal().await?;
        }
        if self.calendar_home_url.is_none() {
            self.discover_calendar_home().await?;
        }
        
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
        let calendars = self.parse_calendars(&body)?;
        
        Ok(calendars)
    }

    fn extract_href(&self, xml: &str, context: &str) -> Option<String> {
        let search_start = if context == "current-user-principal" {
            xml.find("current-user-principal").unwrap_or(0)
        } else if context == "calendar-home-set" {
            xml.find("calendar-home-set").unwrap_or(0)
        } else {
            0
        };
        
        let xml_section = &xml[search_start..];
        
        let href_patterns = [
            "<d:href>",
            "<D:href>",
            "<href>",
            "<href ",
            "<HREF>",
            "<HREF ",
        ];
        
        for start_pattern in &href_patterns {
            if let Some(start) = xml_section.find(start_pattern) {
                let after_tag_name = start + start_pattern.len();
                let content_start = if start_pattern.ends_with(' ') {
                    if let Some(close_bracket) = xml_section[after_tag_name..].find('>') {
                        after_tag_name + close_bracket + 1
                    } else {
                        continue;
                    }
                } else {
                    after_tag_name
                };
                
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
        
        let responses: Vec<&str> = xml.split("<response").collect();
        
        for response in responses.iter().skip(1) {
            if !response.contains("<calendar") && !response.contains("<c:calendar") {
                continue;
            }
            
            let mut url = String::new();
            let mut display_name = String::new();
            
            let href_patterns = [("<href>", "</href>"), ("<href ", "</href>"), ("<d:href>", "</d:href>"), ("<d:href ", "</d:href>")];
            for (start_pattern, end_pattern) in &href_patterns {
                if let Some(start) = response.find(start_pattern) {
                    let after_tag = start + start_pattern.len();
                    let content_start = if start_pattern.ends_with(' ') {
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
            
            let displayname_patterns = [("<displayname>", "</displayname>"), ("<displayname ", "</displayname>"), ("<d:displayname>", "</d:displayname>"), ("<d:displayname ", "</d:displayname>")];
            for (start_pattern, end_pattern) in &displayname_patterns {
                if let Some(start) = response.find(start_pattern) {
                    let after_tag = start + start_pattern.len();
                    let content_start = if start_pattern.ends_with(' ') {
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
                let skip_paths = [
                    "/inbox/",
                    "/outbox/",
                    "/notification/",
                    "/calendars/",
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

    pub async fn get_calendar_ical(&mut self, calendar_name: &str) -> Result<String> {
        let calendars = self.list_calendars().await?;
        
        let calendar = calendars
            .iter()
            .find(|c| c.display_name.contains(calendar_name))
            .ok_or_else(|| anyhow::anyhow!("Calendar '{}' not found", calendar_name))?;
        
        let xml_response = self.get_calendar_events(&calendar.url).await?;
        let events = self.extract_ical_events(&xml_response)?;
        
        if events.is_empty() {
            return Ok(format!(
                "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//iCloud Calendar Private API//EN\r\nCALSCALE:GREGORIAN\r\nX-WR-CALNAME:{}\r\nX-WR-TIMEZONE:UTC\r\nEND:VCALENDAR\r\n",
                calendar.display_name
            ));
        }
        
        let ical = self.combine_ical_events(events, &calendar.display_name);
        Ok(ical)
    }

    async fn get_calendar_events(&self, calendar_url: &str) -> Result<String> {
        let full_url = self.build_url(calendar_url);
        
        let mut headers = self.get_headers();
        headers.insert("Depth", HeaderValue::from_static("1"));
        
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
        Ok(body)
    }

    fn extract_ical_events(&self, xml_response: &str) -> Result<Vec<String>> {
        let mut events = Vec::new();
        
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
                
                let content_start = if start_pattern.contains(' ') || start_pattern.ends_with('=') {
                    if let Some(close_bracket) = xml_response[after_tag_start..].find('>') {
                        after_tag_start + close_bracket + 1
                    } else {
                        pos = after_tag_start;
                        continue;
                    }
                } else {
                    after_tag_start
                };
                
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
                    
                    if ical_data.starts_with("<![CDATA[") {
                        ical_data = ical_data.strip_prefix("<![CDATA[").unwrap_or(&ical_data).to_string();
                        if let Some(cdata_end) = ical_data.rfind("]]>") {
                            ical_data = ical_data[..cdata_end].to_string();
                        }
                    }
                    
                    let unescaped = ical_data
                        .replace("&lt;", "<")
                        .replace("&gt;", ">")
                        .replace("&amp;", "&")
                        .replace("&quot;", "\"")
                        .replace("&#13;", "\r");
                    
                    if unescaped.contains("BEGIN:VCALENDAR") || unescaped.contains("BEGIN:VEVENT") {
                        events.push(unescaped);
                    }
                    
                    pos = end_idx;
                } else {
                    break;
                }
            }
            
            if !events.is_empty() {
                break;
            }
        }
        
        Ok(events)
    }

    fn combine_ical_events(&self, events: Vec<String>, calendar_name: &str) -> String {
        let mut combined = String::new();
        
        combined.push_str("BEGIN:VCALENDAR\r\n");
        combined.push_str("VERSION:2.0\r\n");
        combined.push_str("PRODID:-//iCloud Calendar Private API//EN\r\n");
        combined.push_str("CALSCALE:GREGORIAN\r\n");
        combined.push_str(&format!("X-WR-CALNAME:{}\r\n", calendar_name));
        combined.push_str("X-WR-TIMEZONE:UTC\r\n");
        
        for event in events {
            let mut pos = 0;
            while let Some(start) = event[pos..].find("BEGIN:VEVENT") {
                if let Some(end) = event[pos + start..].find("END:VEVENT") {
                    let vevent = &event[pos + start..pos + start + end + 10];
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
}

# iCloud Calendar REST API

A REST API server for accessing iCloud calendars via HTTP.

## Quick Start

### 1. Create Configuration File

```bash
cp config.example.toml config.toml
```

Edit `config.toml` with your iCloud credentials:

```toml
[icloud]
username = "your-email@icloud.com"
password = "xxxx-xxxx-xxxx-xxxx"  # App-specific password

[server]
host = "127.0.0.1"
port = 8888
```

**Important:** Use an [app-specific password](https://appleid.apple.com), not your regular Apple ID password!

### 2. Build and Run

```bash
# Build the server
cargo build --release

# Run the server
cargo run --bin icloud_calendar_server --release
```

Or use the binary directly:

```bash
./target/release/icloud_calendar_server
```

The server will start on `http://127.0.0.1:8888` by default.

## API Endpoints

### `GET /`
Get API information and available endpoints.

**Response:**
```json
{
  "service": "iCloud Calendar Export API",
  "version": "0.1.0",
  "endpoints": {
    "/": "This help message",
    "/list": "List all available calendars",
    "/calendar/:name": "Get calendar by name (returns iCal format)"
  }
}
```

### `GET /health`
Health check endpoint.

**Response:**
```json
{
  "status": "ok"
}
```

### `GET /list`
List all available calendars.

**Response:**
```json
{
  "calendars": [
    {
      "display_name": "Work",
      "url": "/00000000000/calendars/work/"
    },
    {
      "display_name": "Personal",
      "url": "/00000000000/calendars/home/"
    }
  ],
  "count": 2
}
```

### `GET /calendar/:name`
Get a calendar by name in iCal format.

**Parameters:**
- `name` (path) - Calendar name (partial match)

**Example:**
```bash
curl http://localhost:3000/calendar/Work
```

**Response:**
```
Content-Type: text/calendar; charset=utf-8
Content-Disposition: attachment; filename="Work.ics"

BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//iCloud Calendar Export//EN
CALSCALE:GREGORIAN
X-WR-CALNAME:Work
X-WR-TIMEZONE:UTC
BEGIN:VEVENT
DTSTART:20240115T140000Z
...
END:VEVENT
END:VCALENDAR
```

## Usage Examples

### Using cURL

```bash
# List all calendars
curl http://localhost:8888/list

# Get a specific calendar
curl http://localhost:8888/calendar/Work -o work.ics

# Download calendar
curl http://localhost:8888/calendar/Personal > personal.ics
```

### Using HTTPie

```bash
# List calendars
http :8888/list

# Get calendar
http :8888/calendar/Work
```

### Using a Browser

Simply navigate to:
- http://localhost:8888/ - API info
- http://localhost:8888/list - List calendars
- http://localhost:8888/calendar/Work - Download calendar

### Subscribe in Calendar Apps

Many calendar applications can subscribe to iCal URLs:

1. Get your server URL: `http://localhost:8888/calendar/Work`
2. In your calendar app:
   - **Apple Calendar**: File → New Calendar Subscription
   - **Google Calendar**: Add Calendar → From URL
   - **Outlook**: Add Calendar → Subscribe from web

**Note:** For external access, you'll need to configure port forwarding and use HTTPS.

## Configuration Options

### iCloud Settings

```toml
[icloud]
username = "your-email@icloud.com"
password = "xxxx-xxxx-xxxx-xxxx"
```

### Server Settings

```toml
[server]
# Bind to all interfaces (use for Docker/external access)
host = "0.0.0.0"
port = 8080
```

**Security Warning:** Binding to `0.0.0.0` exposes the server to your network. Make sure to use a reverse proxy with HTTPS in production!

## Deployment

### Using systemd (Linux)

Create `/etc/systemd/system/icloud-calendar-api.service`:

```ini
[Unit]
Description=iCloud Calendar API
After=network.target

[Service]
Type=simple
User=your-username
WorkingDirectory=/path/to/icloud_export/temp
ExecStart=/path/to/icloud_export/temp/target/release/icloud_calendar_server
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable icloud-calendar-api
sudo systemctl start icloud-calendar-api
sudo systemctl status icloud-calendar-api
```

### Using Docker

Create `Dockerfile`:

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin icloud_calendar_server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/icloud_calendar_server /usr/local/bin/
WORKDIR /app
COPY config.toml .
EXPOSE 8888
CMD ["icloud_calendar_server"]
```

Build and run:
```bash
docker build -t icloud-calendar-api .
docker run -p 8888:8888 -v ./config.toml:/app/config.toml icloud-calendar-api
```

### Behind Nginx

```nginx
server {
    listen 80;
    server_name calendar-api.yourdomain.com;

    location / {
        proxy_pass http://127.0.0.1:8888;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

For HTTPS, use Let's Encrypt:
```bash
sudo certbot --nginx -d calendar-api.yourdomain.com
```

## Logging

Set log level with environment variable:

```bash
# Debug logging
RUST_LOG=debug cargo run --bin icloud_calendar_server --release

# Info logging (default)
RUST_LOG=info cargo run --bin icloud_calendar_server --release

# Minimal logging
RUST_LOG=warn cargo run --bin icloud_calendar_server --release
```

## Security Considerations

⚠️ **Important Security Notes:**

1. **Never commit `config.toml`** - It contains your credentials!
2. **Use app-specific passwords** - Not your main Apple ID password
3. **Use HTTPS in production** - Don't expose over plain HTTP externally
4. **Firewall rules** - Restrict access to trusted IPs if possible
5. **Reverse proxy** - Use nginx/Caddy with HTTPS for external access
6. **Rate limiting** - Consider adding rate limiting for public deployments

## Troubleshooting

### Server won't start

```bash
# Check config file exists
ls -la config.toml

# Verify config syntax
cat config.toml

# Check port isn't in use
lsof -i :8888
```

### Authentication errors

- Verify your Apple ID username is correct
- Make sure you're using an app-specific password
- Try generating a new app-specific password

### Can't access from other machines

- Change `host = "0.0.0.0"` in config.toml
- Check firewall rules
- Verify the port is open

## API Client Examples

### JavaScript/TypeScript

```javascript
// List calendars
const response = await fetch('http://localhost:8888/list');
const data = await response.json();
console.log(data.calendars);

// Download calendar
const ical = await fetch('http://localhost:8888/calendar/Work');
const content = await ical.text();
console.log(content);
```

### Python

```python
import requests

# List calendars
response = requests.get('http://localhost:8888/list')
calendars = response.json()['calendars']

# Download calendar
ical = requests.get('http://localhost:8888/calendar/Work')
with open('work.ics', 'w') as f:
    f.write(ical.text)
```

### Go

```go
package main

import (
    "fmt"
    "io"
    "net/http"
)

func main() {
    // List calendars
    resp, _ := http.Get("http://localhost:8888/list")
    defer resp.Body.Close()
    body, _ := io.ReadAll(resp.Body)
    fmt.Println(string(body))

    // Download calendar
    resp2, _ := http.Get("http://localhost:8888/calendar/Work")
    defer resp2.Body.Close()
    ical, _ := io.ReadAll(resp2.Body)
    fmt.Println(string(ical))
}
```

## License

Same as the main project.

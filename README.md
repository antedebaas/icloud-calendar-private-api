# KEEP IN MIND
this is entirely vibecoded and intended for educational purposes. Use at your own risk.
this project is also not intended to be exposed over the internet, so security is not a primary concern. If you want to use it in a production environment, please review the code and implement proper security measures.

# iCloud Calendar Private API

A Rust application that connects to Apple iCloud using CalDAV protocol and exposes private calendar data via REST API and command-line interface.

**Repository:** https://github.com/antedebaas/icloud-calendar-private-api

## Features

- 🔐 Secure authentication with Apple iCloud CalDAV
- 🌐 REST API server with HTTP endpoints
- 💻 Command-line tool for exports
- 📅 Lists all available calendars
- 📥 Exports calendar events as standard iCal format
- 📦 RPM package for Fedora/RHEL/CentOS
- 🔄 Systemd service integration

## Quick Start

### Prerequisites

- Rust 1.70 or later (install from https://rustup.rs)
- An Apple ID with iCloud calendar
- An **app-specific password** from https://appleid.apple.com

⚠️ **Important:** With 2FA enabled (recommended), you MUST use an app-specific password, not your regular Apple ID password.

### Generate App-Specific Password

1. Go to https://appleid.apple.com
2. Sign in with your Apple ID
3. Go to **Security** → **App-Specific Passwords**
4. Click **Generate Password**
5. Label it "iCloud Calendar API"
6. Copy the password (format: `xxxx-xxxx-xxxx-xxxx`)

### Installation

```bash
# Clone the repository
git clone https://github.com/antedebaas/icloud-calendar-private-api.git
cd icloud-calendar-private-api/temp

# Build both binaries
cargo build --release
```

This creates two binaries:
- `icloud-calendar-private-api` - REST API server
- `icloud-calendar-private-cli` - Command-line export tool

---

## REST API Server

### Configuration

Create `config.toml`:

```toml
[icloud]
username = "your-email@icloud.com"
password = "xxxx-xxxx-xxxx-xxxx"  # App-specific password

[server]
host = "127.0.0.1"
port = 8888

[stalwart]
# Enable Stalwart authentication (default: false)
enabled = true
# Stalwart server URL for authentication
server_url = "http://localhost:8080"
# Authentication method: "jmap" or "imap" (default: "jmap")
auth_method = "jmap"
```

**Configuration Sections:**

#### `icloud`
- `username` - Your iCloud Apple ID email
- `password` - Your app-specific password (not your main Apple ID password)

#### `server`
- `host` - Host to bind to (default: `127.0.0.1`)
- `port` - Port to listen on (default: `8888`)
- `public_url` - (Optional) Public URL for API endpoints in responses (e.g., `https://calendar.example.com`). If not set, uses `http://host:port`
- `public_path` - (Optional) Public path prefix for API endpoints (e.g., `api` or `icloud`). If not set, no prefix is used

**Example with reverse proxy:**
```toml
[server]
host = "127.0.0.1"
port = 8888
public_url = "https://calendar.example.com"
public_path = "api"
```
This will make API URLs in the `/list` response show as `https://calendar.example.com/api/calendar/Work` instead of `http://127.0.0.1:8888/calendar/Work`.

#### `stalwart` (Optional)
- `enabled` - Enable/disable Stalwart authentication (default: `false`)
- `server_url` - URL of your Stalwart server (default: `http://localhost:8080`)
- `auth_method` - Authentication method: `jmap` or `imap` (default: `jmap`)

When Stalwart authentication is enabled, the `/list` and `/calendar/:name` endpoints will require HTTP Basic Authentication. The credentials are validated against your Stalwart server.

**Config File Search Order:**
1. `/etc/icloudcalendarapi/config.toml` (system-wide)
2. `config.toml` (current directory)
3. `./config.toml` (explicit current directory)

### Start the Server

```bash
# Using cargo
cargo run --bin icloud-calendar-private-api --release

# Or use the binary directly
./target/release/icloud-calendar-private-api

# Or use the convenience script
./start_server.sh
```

The API will be available at `http://localhost:8888`

### API Endpoints

**Authentication:**

When Stalwart authentication is enabled in the configuration, the `/list` and `/calendar/:name` endpoints require HTTP Basic Authentication. The `/` and `/health` endpoints remain public.

**Authentication Example:**
```bash
# With authentication enabled
curl -u username:password http://localhost:8888/list

# Or using the Authorization header
curl -H "Authorization: Basic $(echo -n 'username:password' | base64)" http://localhost:8888/list
```

If authentication fails, you'll receive a `401 Unauthorized` response:
```json
{
  "error": "Authentication required"
}
```

---

#### `GET /`
Returns API information and available endpoints.

**Example:**
```bash
curl http://localhost:8888/
```

**Response:**
```json
{
  "service": "iCloud Calendar Private API",
  "version": "1.2.0",
  "endpoints": {
    "/": "This help message",
    "/list": "List all available calendars",
    "/calendar/:name": "Get calendar by name (returns iCal format)"
  }
}
```

#### `GET /health`
Health check endpoint.

**Example:**
```bash
curl http://localhost:8888/health
```

**Response:**
```json
{
  "status": "ok"
}
```

#### `GET /list`
List all available calendars with both iCloud URLs and API URLs.

**Example:**
```bash
curl http://localhost:8888/list
```

**Response:**
```json
{
  "calendars": [
    {
      "display_name": "Work",
      "icloud_url": "/XXXXXXXXX/calendars/work/",
      "api_url": "http://127.0.0.1:8888/calendar/Work"
    },
    {
      "display_name": "Personal",
      "icloud_url": "/XXXXXXXXX/calendars/home/",
      "api_url": "http://127.0.0.1:8888/calendar/Personal"
    },
    {
      "display_name": "My Family Calendar",
      "icloud_url": "/XXXXXXXXX/calendars/family/",
      "api_url": "http://127.0.0.1:8888/calendar/My%20Family%20Calendar"
    }
  ],
  "count": 3
}
```

**Note:** The `api_url` field contains the full URL (based on your server configuration) with the properly URL-encoded path to fetch the calendar via this API. If `public_url` or `public_path` are configured in your `config.toml`, those values will be used instead of `http://host:port`. System items like reminders, tasks, inbox, outbox, and notifications are automatically filtered out.

#### `GET /calendar/:name`
Get a calendar by name in iCal format (partial name matching). Returns the calendar data inline as plain iCal.

**Note:** Calendar names with spaces or special characters should be URL-encoded (e.g., `My%20Calendar` for "My Calendar").

**Example:**
```bash
# Get calendar with simple name
curl http://localhost:8888/calendar/Work

# Get calendar with spaces (URL-encoded)
curl http://localhost:8888/calendar/My%20Personal%20Calendar

# Or let your shell handle encoding
curl "http://localhost:8888/calendar/My Personal Calendar"

# Save to file
curl http://localhost:8888/calendar/Work -o work.ics
```

**Response:**
```
Content-Type: text/calendar; charset=utf-8

BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//iCloud Calendar Private API//EN
...
END:VCALENDAR
```

### Subscribe in Calendar Apps

Many calendar applications can subscribe to iCal URLs for automatic updates:

1. Start the API server
2. Get the URL: `http://localhost:8888/calendar/YourCalendarName`
   - For calendar names with spaces, use URL encoding: `http://localhost:8888/calendar/My%20Calendar`
3. Add to your calendar app:
   - **Apple Calendar**: File → New Calendar Subscription
   - **Google Calendar**: Settings → Add Calendar → From URL
   - **Outlook**: Add Calendar → Subscribe from web

**Note:** The calendar is now served inline, making it compatible with more calendar applications and easier to view in browsers.

### Reverse Proxy Setup

When running behind a reverse proxy (nginx, Caddy, Apache), configure `public_url` and optionally `public_path` so the API returns correct URLs in the `/list` endpoint.

#### Example with nginx

**nginx configuration:**
```nginx
server {
    listen 443 ssl;
    server_name calendar.example.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location /api/ {
        proxy_pass http://127.0.0.1:8888/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

**config.toml:**
```toml
[server]
host = "127.0.0.1"
port = 8888
public_url = "https://calendar.example.com"
public_path = "api"
```

**Result:** The `/list` endpoint will return URLs like:
- `https://calendar.example.com/api/calendar/Work`
- `https://calendar.example.com/api/calendar/Personal`

Instead of:
- `http://127.0.0.1:8888/calendar/Work`
- `http://127.0.0.1:8888/calendar/Personal`

This makes it easy for clients to use the API URLs directly without needing to know your internal server configuration.

---

## Command-Line Tool

### Usage

```bash
icloud-calendar-private-cli [OPTIONS] --username <USERNAME> --password <PASSWORD>
```

### Options

```
  -u, --username <USERNAME>  iCloud username (Apple ID)
  -p, --password <PASSWORD>  iCloud password (or app-specific password)
  -l, --list                 List all available calendars with API URLs (metadata only, does not export calendar data)
  -o, --output <OUTPUT>      Output file for iCal data (default: stdout)
  -c, --calendar <CALENDAR>  Calendar name to export (default: all calendars)
  -h, --help                 Print help
  -V, --version              Print version
```

**Note:** The `--list` flag cannot be combined with `--output` or `--calendar` options. Use `--list` to only view available calendars, or use `--calendar`/`--output` to export calendar data.

### Examples

#### List all available calendars

This command **only lists calendar names and URLs** without exporting any calendar data or events.

```bash
./target/release/icloud-calendar-private-cli \
  --username "your-email@icloud.com" \
  --password "xxxx-xxxx-xxxx-xxxx" \
  --list
```

**Output:**
```
🍎 iCloud Calendar Private API - CLI Tool
==========================================

Connecting to iCloud as: your-email@icloud.com

🔍 Discovering CalDAV principal...
✅ Principal URL: /XXXXXXXXX/principal/
🔍 Discovering calendar home...
✅ Calendar Home URL: /XXXXXXXXX/calendars/
📋 Fetching calendar list...

📋 Available Calendars (3)
==========================================

📅 Work
   iCloud URL: /XXXXXXXXX/calendars/work/
   API URL:    /calendar/Work

📅 Personal
   iCloud URL: /XXXXXXXXX/calendars/home/
   API URL:    /calendar/Personal

📅 My Family Calendar
   iCloud URL: /XXXXXXXXX/calendars/family/
   API URL:    /calendar/My%20Family%20Calendar
```

**Note:** The CLI shows relative API URLs (paths only). If you're running the API server, prepend your server URL (e.g., `http://localhost:8888`) to use these paths.

#### Export all calendars to stdout

```bash
./target/release/icloud-calendar-private-cli \
  --username "your-email@icloud.com" \
  --password "xxxx-xxxx-xxxx-xxxx"
```

#### Export to file

```bash
./target/release/icloud-calendar-private-cli \
  -u "your-email@icloud.com" \
  -p "xxxx-xxxx-xxxx-xxxx" \
  -o all_calendars.ics
```

#### Export specific calendar

```bash
./target/release/icloud-calendar-private-cli \
  -u "your-email@icloud.com" \
  -p "xxxx-xxxx-xxxx-xxxx" \
  -c "Work" \
  -o work.ics
```

#### Using environment variables

```bash
export ICLOUD_USERNAME="your-email@icloud.com"
export ICLOUD_PASSWORD="xxxx-xxxx-xxxx-xxxx"

./target/release/icloud-calendar-private-cli \
  -u "$ICLOUD_USERNAME" \
  -p "$ICLOUD_PASSWORD" \
  -c "Personal" \
  -o personal.ics
```

### Automation with Cron

```bash
# Edit crontab
crontab -e

# Add daily backup at 2 AM
0 2 * * * /usr/bin/icloud-calendar-private-cli -u "user@email.com" -p "xxxx-xxxx-xxxx-xxxx" -o "/backups/calendar-$(date +\%Y\%m\%d).ics"
```

---

## RPM Installation

### Install from RPM

```bash
# Install the package
sudo dnf install icloud-calendar-private-api

# Or from COPR
sudo dnf copr enable antedebaas/icloud-calendar-private-api
sudo dnf install icloud-calendar-private-api
```

### What Gets Installed

- **Server binary:** `/usr/bin/icloud-calendar-private-api`
- **CLI binary:** `/usr/bin/icloud-calendar-private-cli`
- **Systemd service:** `/usr/lib/systemd/system/icloud-calendar-private-api.service`
- **Config directory:** `/etc/icloudcalendarapi/`
- **Example config:** `/etc/icloudcalendarapi/config.example.toml`
- **System user:** `icloudcalendarapi`

### Post-Installation Setup

```bash
# 1. Copy example config
sudo cp /etc/icloudcalendarapi/config.example.toml /etc/icloudcalendarapi/config.toml

# 2. Edit configuration
sudo nano /etc/icloudcalendarapi/config.toml

# 3. Set permissions
sudo chown icloudcalendarapi:icloudcalendarapi /etc/icloudcalendarapi/config.toml
sudo chmod 600 /etc/icloudcalendarapi/config.toml

# 4. Enable and start service
sudo systemctl enable --now icloud-calendar-private-api.service

# 5. Check status
sudo systemctl status icloud-calendar-private-api.service

# 6. View logs
sudo journalctl -u icloud-calendar-private-api.service -f
```

### Verify Installation

```bash
# Test API
curl http://localhost:8888/list

# Use CLI tool
icloud-calendar-private-cli --help
```

### Firewall Configuration

```bash
# Open port 8888 (only if needed for remote access)
sudo firewall-cmd --permanent --add-port=8888/tcp
sudo firewall-cmd --reload
```

---

## Docker Deployment

### Dockerfile

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin icloud-calendar-private-api

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/icloud-calendar-private-api /usr/local/bin/
WORKDIR /app
COPY config.toml .
EXPOSE 8888
CMD ["icloud-calendar-private-api"]
```

### Build and Run

```bash
# Build image
docker build -t icloud-calendar-api .

# Run container
docker run -p 8888:8888 -v ./config.toml:/app/config.toml icloud-calendar-api
```

---

## Development

### Project Structure

```
temp/
├── src/
│   ├── lib.rs              # iCloud CalDAV client library
│   ├── main.rs             # CLI tool
│   └── server.rs           # REST API server
├── Cargo.toml              # Rust dependencies
├── config.example.toml     # Example configuration
├── icloud-calendar-private-api.spec    # RPM spec file
└── icloud-calendar-private-api.service # Systemd service
```

### Build from Source

```bash
# Build both binaries
cargo build --release

# Build only the server
cargo build --release --bin icloud-calendar-private-api

# Build only the CLI
cargo build --release --bin icloud-calendar-private-cli

# Run tests
cargo test

# Check code
cargo check
```

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=debug cargo run --bin icloud-calendar-private-api
```

---

## Technical Details

### How It Works

1. **CalDAV Protocol**: Uses Apple's official CalDAV protocol
2. **Discovery**: Automatically discovers principal and calendar home URLs
3. **Authentication**: HTTP Basic Auth with app-specific passwords
4. **PROPFIND**: Lists available calendars
5. **REPORT**: Queries calendar events
6. **iCal Export**: Converts CalDAV data to standard RFC 5545 iCal format

### CalDAV Endpoint

- **Base URL:** `https://caldav.icloud.com/`
- **Protocol:** CalDAV (WebDAV extension)
- **Authentication:** HTTP Basic Auth
- **Transport:** HTTPS

### Supported Features

- ✅ Event titles, descriptions, locations
- ✅ Date and time information (with timezones)
- ✅ Recurrence rules
- ✅ Attendees and organizers
- ✅ Alarms and reminders
- ✅ Multiple calendars
- ✅ System and shared calendars (filtered out)

---

## Troubleshooting

### Authentication Errors (401)

- Verify your Apple ID email is correct
- Ensure you're using an **app-specific password**
- Generate a new app-specific password
- Check for typos in credentials

### No Calendars Found

- Log into iCloud.com and verify calendars exist
- Some shared calendars may not be accessible via CalDAV
- Check service logs for detailed errors

### Server Won't Start

```bash
# Check config file exists
ls -l /etc/icloudcalendarapi/config.toml
ls -l config.toml

# Verify config syntax
cat config.toml

# Check port isn't in use
lsof -i :8888

# View detailed logs
RUST_LOG=debug ./target/release/icloud-calendar-private-api
```

### Connection Errors

- Check internet connection
- Verify you can access https://caldav.icloud.com/
- Check firewall rules
- Some corporate networks may block CalDAV traffic

### Permission Errors (RPM Installation)

```bash
# Fix config ownership
sudo chown icloudcalendarapi:icloudcalendarapi /etc/icloudcalendarapi/config.toml
sudo chmod 600 /etc/icloudcalendarapi/config.toml

# Fix directory permissions
sudo chown -R icloudcalendarapi:icloudcalendarapi /etc/icloudcalendarapi
sudo chown -R icloudcalendarapi:icloudcalendarapi /var/lib/icloudcalendarapi
```

---

## Security Considerations

⚠️ **Important Security Notes:**

- **Never commit `config.toml`** with credentials to version control
- **Use app-specific passwords** - not your main Apple ID password
- **Enable Stalwart authentication** to protect API endpoints (see Configuration section)
- **Don't expose the API to the internet** without HTTPS and authentication
- **Use a reverse proxy** (nginx/Caddy) with HTTPS for external access
- **Restrict firewall access** to trusted IPs only
- **Monitor access logs** regularly
- **Rotate passwords** periodically
- **Set proper file permissions** on config files (`chmod 600`)

### Authentication

This project now includes optional Stalwart authentication:

- When enabled, `/list` and `/calendar/:name` endpoints require HTTP Basic Auth
- Credentials are validated against your Stalwart mail server
- Supports both JMAP and IMAP authentication methods
- Public endpoints (`/` and `/health`) remain unauthenticated for monitoring

**To enable authentication**, add the `[stalwart]` section to your `config.toml` (see Configuration section above).

### For Production Use

If you choose to use this in production (not recommended without review):

1. ✅ Use HTTPS with valid certificates
2. ✅ Implement rate limiting
3. ✅ Add authentication to API endpoints
4. ✅ Monitor and log all access
5. ✅ Use secrets management (not config files)
6. ✅ Regular security audits
7. ✅ Keep dependencies updated

---

## Use Cases

### 1. Calendar Backups

```bash
# Daily automated backups with CLI
0 2 * * * /usr/bin/icloud-calendar-private-cli -u "$USER" -p "$PASS" -o "/backups/cal-$(date +\%Y\%m\%d).ics"
```

### 2. Calendar Integration

```bash
# Start API server, integrate with other services
curl http://localhost:8888/list | jq .
```

### 3. Calendar Migration

```bash
# Export from iCloud
./target/release/icloud-calendar-private-cli -u "old@email.com" -p "xxxx" -o export.ics

# Import to Google Calendar, Outlook, etc.
```

### 4. Calendar Subscriptions

```
# Subscribe in calendar apps
http://your-server:8888/calendar/Work
```

---

## Comparison: API vs CLI

| Feature | REST API Server | CLI Tool |
|---------|----------------|----------|
| **Use Case** | Continuous access | One-time exports |
| **Integration** | HTTP endpoints | Shell scripts |
| **Scheduling** | Always running | Cron jobs |
| **Multiple Clients** | Yes | No |
| **Auto-updates** | Yes (subscriptions) | No |
| **Resource Usage** | Always running | Only when executed |
| **Best For** | Services, apps | Backups, migrations |

**Use the API Server when:**
- You need continuous calendar access
- Multiple clients need to access calendars
- You want calendar subscriptions
- Integrating with other services

**Use the CLI Tool when:**
- You need one-time exports
- Creating backup scripts
- Migrating calendars
- Running scheduled exports

---

## Contributing

This is an educational project. If you want to contribute:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

---

## License

MIT License - See LICENSE file for details

---

## Disclaimer

This tool is not affiliated with, authorized, maintained, sponsored, or endorsed by Apple Inc. or any of its affiliates or subsidiaries. This is an independent project that uses publicly documented CalDAV protocols.

Use at your own risk. The author assumes no liability for any damages or losses.

---

## Support

For issues or questions:
- **GitHub Issues:** https://github.com/antedebaas/icloud-calendar-private-api/issues
- **Check logs:** `journalctl -u icloud-calendar-private-api.service -f`
- **Debug mode:** `RUST_LOG=debug ./target/release/icloud-calendar-private-api`

---

## Changelog

### v1.2.0 (Current)
- 🔒 **NEW:** Stalwart authentication support for API endpoints
  - Optional HTTP Basic Authentication for `/list` and `/calendar/:name` endpoints
  - Validates credentials against Stalwart mail server
  - Supports both JMAP and IMAP authentication methods
  - Public endpoints (`/` and `/health`) remain accessible without authentication
- ✨ `/list` endpoint now returns full URLs in `api_url` field (based on server configuration)
- ✨ Added `public_url` and `public_path` configuration for reverse proxy support
- 🔧 Reminders/tasks are now filtered out from calendar listings (not actual calendars)
- 🔧 Updated dependencies for improved security

### v1.1.0
- ✨ Calendar endpoint now serves iCal data inline instead of as attachment
- ✨ Added support for URL-encoded calendar names (handles spaces and special characters)
- ✨ List endpoint now includes API URLs alongside iCloud URLs for easier integration
- ✨ CLI tool now supports `--list` flag to show calendars with API URLs
- 🔧 Improved compatibility with calendar applications and browsers

### v1.0.0 (Initial Release)
- REST API server with HTTP endpoints
- Command-line export tool
- RPM package for Fedora/RHEL/CentOS
- Systemd service integration
- CalDAV protocol implementation
- iCal (RFC 5545) export format
- Multi-calendar support

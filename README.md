# iCloud Calendar Export Tool

A Rust application that connects to Apple iCloud using CalDAV protocol and exports private calendar data as iCal format.

**NEW:** Now includes a REST API server! See [API.md](API.md) for details.

## Features

- 🔐 Secure authentication with Apple iCloud CalDAV
- 📅 Lists all available calendars
- 🔍 Filter calendars by name
- 📥 Exports calendar events as standard iCal format
- 💾 Save to file or output to stdout
- ✨ Clean, user-friendly CLI interface

## Prerequisites

- Rust 1.70 or later
- An Apple ID (iCloud account)
- App-specific password (recommended) or account password

## Quick Start

### Command Line Tool

1. Clone or download this project
2. Navigate to the project directory:
   ```bash
   cd temp
   ```

3. Build the project:
   ```bash
   cargo build --release
   ```

### REST API Server

See [API.md](API.md) for complete API documentation.

```bash
# Create config file
cp config.example.toml config.toml
# Edit config.toml with your credentials

# Run the server
./start_server.sh
# Or: cargo run --bin icloud_calendar_server --release
```

The API will be available at `http://localhost:8888`

## Installation

1. Clone or download this project
2. Navigate to the project directory:
   ```bash
   cd temp
   ```

3. Build the project:
   ```bash
   cargo build --release
   ```

## Important: Authentication Setup

### Two-Factor Authentication (2FA)

If you have two-factor authentication enabled on your Apple ID (which is highly recommended), you **must** use an app-specific password instead of your regular password.

### Generating an App-Specific Password

1. Go to [appleid.apple.com](https://appleid.apple.com)
2. Sign in with your Apple ID
3. Navigate to **Security** section
4. Under **App-Specific Passwords**, click **Generate Password**
5. Enter a label (e.g., "iCloud Calendar Export")
6. Copy the generated password (format: `xxxx-xxxx-xxxx-xxxx`)
7. Use this password with the tool

**Note:** App-specific passwords are different from your regular Apple ID password and provide secure, limited access to your iCloud data.

## Usage

### Basic Usage (Export all calendars to stdout)

```bash
cargo run -- --username "your-apple-id@icloud.com" --password "your-app-specific-password"
```

Or with the compiled binary:

```bash
./target/release/icloud_calendar_export \
  --username "your-apple-id@icloud.com" \
  --password "xxxx-xxxx-xxxx-xxxx"
```

### Export to File

```bash
cargo run -- \
  --username "your-apple-id@icloud.com" \
  --password "xxxx-xxxx-xxxx-xxxx" \
  --output my_calendar.ics
```

### Export Specific Calendar

```bash
cargo run -- \
  --username "your-apple-id@icloud.com" \
  --password "xxxx-xxxx-xxxx-xxxx" \
  --calendar "Work" \
  --output work_calendar.ics
```

### Command Line Options

```
Options:
  -u, --username <USERNAME>    iCloud username (Apple ID)
  -p, --password <PASSWORD>    iCloud password (or app-specific password)
  -o, --output <OUTPUT>        Output file for iCal data (default: stdout)
  -c, --calendar <CALENDAR>    Calendar name to export (default: all calendars)
  -h, --help                   Print help
  -V, --version                Print version
```

## How It Works

This tool uses the CalDAV protocol to communicate with iCloud:

1. **Discovery Phase**: Discovers the CalDAV principal and calendar home URLs
2. **List Calendars**: Retrieves all available calendars for your account
3. **Fetch Events**: Downloads all events from selected calendar(s)
4. **Export**: Converts the data to standard iCal (`.ics`) format

The CalDAV protocol is the standard way to access calendar data and is officially supported by Apple.

## Output Format

The tool outputs data in the standard iCalendar (RFC 5545) format, which includes:

- Calendar metadata (name, timezone, etc.)
- Event details (title, description, location)
- Date and time information
- Recurrence rules
- Attendees and organizers
- Alarms and reminders

The generated `.ics` files can be imported into:
- Apple Calendar
- Google Calendar
- Microsoft Outlook
- Mozilla Thunderbird
- Any other calendar application supporting iCal format

## Example Output

```ical
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//iCloud Calendar Export//EN
CALSCALE:GREGORIAN
X-WR-CALNAME:Personal
X-WR-TIMEZONE:UTC
BEGIN:VEVENT
DTSTART:20240115T140000Z
DTEND:20240115T150000Z
SUMMARY:Team Meeting
DESCRIPTION:Weekly sync with the team
LOCATION:Conference Room A
UID:unique-event-id@icloud.com
END:VEVENT
END:VCALENDAR
```

## Security Considerations

- **Never commit your password or app-specific password to version control**
- Use environment variables for credentials in scripts:
  ```bash
  export ICLOUD_USERNAME="your-email@icloud.com"
  export ICLOUD_PASSWORD="xxxx-xxxx-xxxx-xxxx"
  cargo run -- --username "$ICLOUD_USERNAME" --password "$ICLOUD_PASSWORD"
  ```
- App-specific passwords can be revoked at any time from your Apple ID settings
- The tool uses HTTPS for all communications with iCloud

## Troubleshooting

### Enable Debug Mode

If you're experiencing issues, enable debug mode to see the raw XML responses:

```bash
DEBUG=1 cargo run --release -- \
  --username "your-email@icloud.com" \
  --password "xxxx-xxxx-xxxx-xxxx"
```

This will show you the actual responses from iCloud's servers, which can help diagnose issues.

### Authentication Errors

- **Error: "Failed to discover principal: 401"**
  - Your username or password is incorrect
  - If you have 2FA enabled, make sure you're using an app-specific password
  - Check that your Apple ID is correct (usually ends with @icloud.com, @me.com, or @mac.com)

- **Error: "Could not find principal URL in response"**
  - Enable debug mode (see above) to see the actual response
  - This may indicate an authentication issue or unexpected response format
  - Verify your credentials are correct

### Connection Errors

- **Error: "Connection timeout" or "Network error"**
  - Check your internet connection
  - Verify that you can access iCloud.com in your browser
  - Some corporate networks may block CalDAV traffic

### No Calendars Found

- Check that you have calendars set up in your iCloud account
- Log into iCloud.com and verify your calendars are visible there

### Empty Calendar Export

- The calendar might not have any events
- Check the date range (this tool exports all events by default)

## Technical Details

### CalDAV Protocol

The tool implements the following CalDAV operations:
- **PROPFIND**: Discovers calendar resources and properties
- **REPORT**: Queries calendar events with filters

### iCloud CalDAV Endpoint

- Base URL: `https://caldav.icloud.com/`
- Authentication: HTTP Basic Auth
- Supported: All standard CalDAV operations

## License

This project is provided as-is for educational and personal use.

## Contributing

Feel free to submit issues or pull requests if you find bugs or want to add features.

## Disclaimer

This tool is not affiliated with, authorized, maintained, sponsored, or endorsed by Apple Inc. or any of its affiliates or subsidiaries. This is an independent project that uses publicly documented CalDAV protocols.

# Quick Start Guide

Get started with iCloud Calendar Export in 5 minutes!

## Prerequisites

- Rust installed (visit https://rustup.rs if you don't have it)
- An Apple ID with iCloud calendar
- An app-specific password (see below)

## Step 1: Get Your App-Specific Password

**⚠️ IMPORTANT:** If you have two-factor authentication enabled (recommended), you MUST use an app-specific password.

1. Go to https://appleid.apple.com
2. Sign in
3. Go to **Security** → **App-Specific Passwords**
4. Click **Generate Password**
5. Label it "iCloud Calendar Export"
6. Copy the password (format: `xxxx-xxxx-xxxx-xxxx`)

## Step 2: Build the Application

```bash
cd temp
cargo build --release
```

This will take a few minutes the first time as it downloads and compiles dependencies.

## Step 3: Run Your First Export

### Test it out (prints to screen):

```bash
cargo run --release -- \
  --username "your-email@icloud.com" \
  --password "xxxx-xxxx-xxxx-xxxx"
```

### Export to a file:

```bash
cargo run --release -- \
  --username "your-email@icloud.com" \
  --password "xxxx-xxxx-xxxx-xxxx" \
  --output my_calendar.ics
```

### Export a specific calendar:

```bash
cargo run --release -- \
  --username "your-email@icloud.com" \
  --password "xxxx-xxxx-xxxx-xxxx" \
  --calendar "Work" \
  --output work_calendar.ics
```

## Step 4: Use the Exported Calendar

The `.ics` file can be:
- Imported into any calendar application
- Opened directly in Calendar apps
- Shared with others
- Backed up for safekeeping

### Import into Google Calendar:
1. Go to Google Calendar
2. Click the **+** next to "Other calendars"
3. Select **Import**
4. Upload your `.ics` file

### Import into Apple Calendar:
```bash
open my_calendar.ics
```

### Import into Outlook:
1. Open Outlook
2. Go to **File** → **Open & Export** → **Import/Export**
3. Select **Import an iCalendar (.ics) or vCalendar file**
4. Browse to your `.ics` file

## Common Use Cases

### One-Time Export

Just run the command once to create a backup:

```bash
./target/release/icloud_calendar_export \
  -u "your-email@icloud.com" \
  -p "xxxx-xxxx-xxxx-xxxx" \
  -o "backup_$(date +%Y%m%d).ics"
```

### Automated Daily Backups

See `AUTOMATION.md` for detailed instructions on setting up:
- Cron jobs
- Systemd timers
- Cloud sync integration

### Migration to Another Service

1. Export all calendars:
   ```bash
   ./target/release/icloud_calendar_export \
     -u "your-email@icloud.com" \
     -p "xxxx-xxxx-xxxx-xxxx" \
     -o full_backup.ics
   ```

2. Import into your new calendar service

### Share Calendar with Non-iCloud Users

1. Export specific calendar:
   ```bash
   ./target/release/icloud_calendar_export \
     -u "your-email@icloud.com" \
     -p "xxxx-xxxx-xxxx-xxxx" \
     -c "Shared Events" \
     -o shared.ics
   ```

2. Send the `.ics` file to others

## Troubleshooting

### Enable Debug Mode

If something isn't working, see what's actually happening:

```bash
DEBUG=1 cargo run --release -- \
  --username "your-email@icloud.com" \
  --password "xxxx-xxxx-xxxx-xxxx"
```

This shows the raw XML responses from iCloud.

### "Authentication failed" (401 error)
- Double-check your Apple ID email
- Make sure you're using an **app-specific password** if you have 2FA enabled
- The password should be in format: `xxxx-xxxx-xxxx-xxxx`

### "No calendars found"
- Log into iCloud.com and verify you have calendars set up
- Some shared calendars may not be accessible via CalDAV

### Build errors
- Make sure you have Rust installed: `rustc --version`
- Update Rust: `rustup update`
- Clean and rebuild: `cargo clean && cargo build --release`

## Next Steps

- **Security:** Read `README.md` section on secure credential handling
- **Automation:** Check out `AUTOMATION.md` for scheduled backups
- **Examples:** Review `export_example.sh` for script templates

## Quick Command Reference

```bash
# Show help
cargo run --release -- --help

# Export all calendars to file
cargo run --release -- -u USER -p PASS -o output.ics

# Export specific calendar
cargo run --release -- -u USER -p PASS -c "Calendar Name" -o output.ics

# Use the compiled binary directly (faster)
./target/release/icloud_calendar_export -u USER -p PASS -o output.ics
```

## Security Tips

✅ **DO:**
- Use app-specific passwords
- Store credentials in environment variables or password managers
- Set file permissions: `chmod 600 credentials_file`
- Revoke app-specific passwords when no longer needed

❌ **DON'T:**
- Commit passwords to version control
- Share your app-specific password
- Use your main Apple ID password
- Store credentials in plain text files with open permissions

## Getting Help

- Check `README.md` for detailed documentation
- Review `AUTOMATION.md` for automation examples
- File issues on the project repository
- Verify your setup: https://appleid.apple.com

---

**Happy exporting! 🎉**

# Automated Backup Setup

This guide shows how to set up automated calendar backups using cron or systemd timers.

## Option 1: Using Cron (Simple)

### 1. Create a backup script

Create a file at `~/.local/bin/icloud-calendar-backup.sh`:

```bash
#!/bin/bash

# Configuration
ICLOUD_USERNAME="your-email@icloud.com"
ICLOUD_PASSWORD="xxxx-xxxx-xxxx-xxxx"
BACKUP_DIR="$HOME/icloud-backups"
TOOL_PATH="$HOME/path/to/icloud_calendar_export"

# Create backup directory if it doesn't exist
mkdir -p "$BACKUP_DIR"

# Create timestamped filename
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
OUTPUT_FILE="$BACKUP_DIR/calendar_backup_$TIMESTAMP.ics"

# Run the export
cd "$TOOL_PATH"
cargo run --release -- \
  --username "$ICLOUD_USERNAME" \
  --password "$ICLOUD_PASSWORD" \
  --output "$OUTPUT_FILE"

# Optional: Delete backups older than 30 days
find "$BACKUP_DIR" -name "calendar_backup_*.ics" -mtime +30 -delete

echo "Backup completed: $OUTPUT_FILE"
```

### 2. Make the script executable

```bash
chmod 700 ~/.local/bin/icloud-calendar-backup.sh
```

### 3. Add to crontab

```bash
crontab -e
```

Add one of these lines:

```cron
# Daily backup at 2 AM
0 2 * * * /home/your-username/.local/bin/icloud-calendar-backup.sh

# Weekly backup (Sunday at 3 AM)
0 3 * * 0 /home/your-username/.local/bin/icloud-calendar-backup.sh

# Every 6 hours
0 */6 * * * /home/your-username/.local/bin/icloud-calendar-backup.sh
```

## Option 2: Using Systemd Timer (Recommended for Linux)

### 1. Create the service file

Create `/etc/systemd/user/icloud-calendar-backup.service`:

```ini
[Unit]
Description=iCloud Calendar Backup
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
Environment="ICLOUD_USERNAME=your-email@icloud.com"
Environment="ICLOUD_PASSWORD=xxxx-xxxx-xxxx-xxxx"
Environment="BACKUP_DIR=%h/icloud-backups"
ExecStartPre=/bin/mkdir -p %h/icloud-backups
ExecStart=/home/your-username/path/to/target/release/icloud_calendar_export --username %ICLOUD_USERNAME% --password %ICLOUD_PASSWORD% --output %BACKUP_DIR%/calendar_%Y%m%d_%H%M%S.ics
```

### 2. Create the timer file

Create `/etc/systemd/user/icloud-calendar-backup.timer`:

```ini
[Unit]
Description=iCloud Calendar Backup Timer
Requires=icloud-calendar-backup.service

[Timer]
# Run daily at 2 AM
OnCalendar=*-*-* 02:00:00
# Run on boot if we missed a scheduled time
Persistent=true

[Install]
WantedBy=timers.target
```

### 3. Enable and start the timer

```bash
systemctl --user enable icloud-calendar-backup.timer
systemctl --user start icloud-calendar-backup.timer

# Check status
systemctl --user status icloud-calendar-backup.timer

# View logs
journalctl --user -u icloud-calendar-backup.service
```

## Security Best Practices

### Using a Separate Credentials File

Instead of hardcoding credentials in scripts:

1. Create a credentials file with restricted permissions:

```bash
cat > ~/.icloud-credentials << 'EOF'
USERNAME=your-email@icloud.com
PASSWORD=xxxx-xxxx-xxxx-xxxx
EOF

chmod 600 ~/.icloud-credentials
```

2. Source it in your backup script:

```bash
#!/bin/bash
source ~/.icloud-credentials

cargo run --release -- \
  --username "$USERNAME" \
  --password "$PASSWORD" \
  --output "backup_$(date +%Y%m%d).ics"
```

### Using Pass (Password Manager)

If you use the `pass` password manager:

```bash
#!/bin/bash

ICLOUD_USERNAME="your-email@icloud.com"
ICLOUD_PASSWORD=$(pass show icloud/app-specific-password)

cargo run --release -- \
  --username "$ICLOUD_USERNAME" \
  --password "$ICLOUD_PASSWORD" \
  --output "backup_$(date +%Y%m%d).ics"
```

### Using macOS Keychain

On macOS, you can use the keychain:

```bash
#!/bin/bash

ICLOUD_USERNAME="your-email@icloud.com"
ICLOUD_PASSWORD=$(security find-generic-password -a "$ICLOUD_USERNAME" -s "icloud-backup" -w)

./target/release/icloud_calendar_export \
  --username "$ICLOUD_USERNAME" \
  --password "$ICLOUD_PASSWORD" \
  --output "backup_$(date +%Y%m%d).ics"
```

First, add the password to keychain:

```bash
security add-generic-password -a "your-email@icloud.com" -s "icloud-backup" -w "xxxx-xxxx-xxxx-xxxx"
```

## Cloud Backup Integration

### Sync to Dropbox/Google Drive

```bash
#!/bin/bash

# Export calendar
OUTPUT_FILE="$HOME/Dropbox/icloud-calendar-backup.ics"

cargo run --release -- \
  --username "$ICLOUD_USERNAME" \
  --password "$ICLOUD_PASSWORD" \
  --output "$OUTPUT_FILE"
```

### Encrypt and backup to S3

```bash
#!/bin/bash

TEMP_FILE="/tmp/calendar_backup_$(date +%Y%m%d).ics"

# Export
cargo run --release -- \
  --username "$ICLOUD_USERNAME" \
  --password "$ICLOUD_PASSWORD" \
  --output "$TEMP_FILE"

# Encrypt with GPG
gpg --encrypt --recipient your@email.com "$TEMP_FILE"

# Upload to S3
aws s3 cp "${TEMP_FILE}.gpg" "s3://your-bucket/backups/calendar_$(date +%Y%m%d).ics.gpg"

# Cleanup
rm "$TEMP_FILE" "${TEMP_FILE}.gpg"
```

## Monitoring and Notifications

### Send email on failure

```bash
#!/bin/bash

if ! cargo run --release -- --username "$USER" --password "$PASS" --output "backup.ics"; then
    echo "iCloud calendar backup failed!" | mail -s "Backup Failed" your@email.com
fi
```

### Desktop notification (Linux)

```bash
#!/bin/bash

if cargo run --release -- --username "$USER" --password "$PASS" --output "backup.ics"; then
    notify-send "iCloud Backup" "Calendar backup completed successfully"
else
    notify-send -u critical "iCloud Backup" "Backup failed!"
fi
```

## Backup Retention Policy

Example script with retention management:

```bash
#!/bin/bash

BACKUP_DIR="$HOME/icloud-backups"
MAX_BACKUPS=30  # Keep last 30 backups

# Create new backup
cargo run --release -- \
  --username "$ICLOUD_USERNAME" \
  --password "$ICLOUD_PASSWORD" \
  --output "$BACKUP_DIR/calendar_$(date +%Y%m%d_%H%M%S).ics"

# Keep only the most recent backups
ls -t "$BACKUP_DIR"/calendar_*.ics | tail -n +$((MAX_BACKUPS + 1)) | xargs rm -f
```

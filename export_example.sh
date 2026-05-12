#!/bin/bash

# iCloud Calendar Export - Example Script
# This demonstrates secure credential handling using environment variables

# Set your credentials here or load from a secure credential manager
# NEVER commit this file with actual credentials!

# Option 1: Set environment variables directly
export ICLOUD_USERNAME="your-apple-id@icloud.com"
export ICLOUD_PASSWORD="xxxx-xxxx-xxxx-xxxx"

# Option 2: Read from a secure file (ensure file has restricted permissions: chmod 600)
# export ICLOUD_USERNAME=$(cat ~/.icloud_credentials | grep USERNAME | cut -d= -f2)
# export ICLOUD_PASSWORD=$(cat ~/.icloud_credentials | grep PASSWORD | cut -d= -f2)

# Option 3: Prompt for password (most secure for interactive use)
# echo "Enter your iCloud username:"
# read ICLOUD_USERNAME
# echo "Enter your app-specific password:"
# read -s ICLOUD_PASSWORD
# export ICLOUD_USERNAME
# export ICLOUD_PASSWORD

# Build the project (only needed once)
# cargo build --release

# Run the export
cargo run --release -- \
  --username "$ICLOUD_USERNAME" \
  --password "$ICLOUD_PASSWORD" \
  --output "backup_$(date +%Y%m%d).ics"

# Or to export a specific calendar:
# cargo run --release -- \
#   --username "$ICLOUD_USERNAME" \
#   --password "$ICLOUD_PASSWORD" \
#   --calendar "Work" \
#   --output "work_calendar.ics"

# Clean up environment variables after use
unset ICLOUD_USERNAME
unset ICLOUD_PASSWORD

echo "Export complete!"

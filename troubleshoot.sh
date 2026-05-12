#!/bin/bash

# iCloud Calendar Export - Troubleshooting Script
# This script helps diagnose connection and authentication issues

echo "🔧 iCloud Calendar Export Troubleshooting"
echo "=========================================="
echo ""

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ ERROR: Rust/Cargo is not installed"
    echo "   Install from: https://rustup.rs"
    exit 1
fi

echo "✅ Rust is installed: $(rustc --version)"
echo ""

# Get credentials
read -p "Enter your iCloud username (Apple ID): " ICLOUD_USERNAME
echo "Enter your app-specific password (will not be displayed):"
read -s ICLOUD_PASSWORD
echo ""
echo ""

# Test 1: Build the project
echo "📦 Test 1: Building the project..."
if cargo build --release 2>&1 | tail -5; then
    echo "✅ Build successful"
else
    echo "❌ Build failed"
    exit 1
fi
echo ""

# Test 2: Check network connectivity
echo "🌐 Test 2: Checking network connectivity to iCloud..."
if curl -s -o /dev/null -w "%{http_code}" https://caldav.icloud.com/ | grep -q "401\|403\|200"; then
    echo "✅ Can reach iCloud CalDAV server"
else
    echo "❌ Cannot reach iCloud CalDAV server"
    echo "   Check your internet connection"
    exit 1
fi
echo ""

# Test 3: Run with debug mode
echo "🔍 Test 3: Testing authentication with DEBUG mode..."
echo "This will show the raw XML responses from iCloud."
echo ""
echo "Press Enter to continue..."
read

DEBUG=1 cargo run --release -- \
  --username "$ICLOUD_USERNAME" \
  --password "$ICLOUD_PASSWORD" 2>&1

echo ""
echo "=========================================="
echo ""
echo "📋 Common Issues:"
echo ""
echo "1. If you see '401 Unauthorized':"
echo "   - Verify your username is correct"
echo "   - Make sure you're using an APP-SPECIFIC password"
echo "   - Generate one at: https://appleid.apple.com"
echo ""
echo "2. If the XML response is empty or shows an error:"
echo "   - Your credentials might be incorrect"
echo "   - Your Apple ID might not have calendars set up"
echo ""
echo "3. If you see 'Could not find principal URL':"
echo "   - Check the DEBUG output above for the actual XML"
echo "   - The response format might be unexpected"
echo "   - This could indicate an iCloud service change"
echo ""
echo "4. If everything works in debug but fails otherwise:"
echo "   - This is unusual - file a bug report with the DEBUG output"
echo ""
echo "For more help, check README.md or file an issue on the repository."

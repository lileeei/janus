#!/bin/bash

# Janus Chrome Demo Script

echo "==== Janus Chrome Demo ===="
echo "1. Starting Chrome with remote debugging..."

# Start Chrome in background with remote debugging enabled
case "$(uname -s)" in
   Darwin)
     # macOS
     open -a "Google Chrome" --args --remote-debugging-port=9222 --headless=new
     ;;
   Linux)
     # Linux
     google-chrome --remote-debugging-port=9222 --headless=new &
     ;;
   CYGWIN*|MINGW*|MSYS*)
     # Windows
     start chrome.exe --remote-debugging-port=9222 --headless=new
     ;;
   *)
     echo "Unsupported operating system"
     exit 1
     ;;
esac

echo "2. Waiting for Chrome to initialize..."
sleep 3

echo "3. Running the minimal_chrome example..."
cargo run --example minimal_chrome

echo "\nDemo completed."
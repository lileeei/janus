#!/bin/bash

# Simple script to launch Chrome with remote debugging enabled

echo "Starting Chrome with remote debugging on port 9222..."

# Detect OS
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

echo "Chrome started successfully. Keep this terminal open and run examples in another terminal."
echo "Example: cargo run --example minimal_chrome"
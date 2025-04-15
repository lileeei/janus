#!/bin/bash

# Helper script to clean and run the minimal Chrome example

echo "Cleaning project..."
cargo clean

echo "Starting Chrome with remote debugging..."
# Start Chrome with remote debugging in the background
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

echo "Waiting for Chrome to start..."
sleep 3

echo "Building and running example..."
cargo build --example minimal_chrome
cargo run --example minimal_chrome

echo "Example completed."
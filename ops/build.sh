#!/bin/bash

# LogLine Build Script - Documents the build process as spans

set -e

# Check if we have an identity
ID_EXISTS=$(cargo run -- whoami 2>/dev/null | grep -c "Current identity" || echo "0")

if [ "$ID_EXISTS" == "0" ]; then
    echo "No LogLine ID found, creating a bootstrap identity..."
    
    # Generate a hostname-based node name
    NODE_NAME="$(hostname)-bootstrap"
    
    # Create a LogLine ID (this will also record a genesis span)
    cargo run -- init "$NODE_NAME"
else
    echo "LogLine ID exists, using existing identity"
fi

# Record a span for the build process
cargo run -- span "Building LogLine from source code"

# Build the project
echo "Building LogLine..."
cargo build --release

# Record a span for the successful build
if [ $? -eq 0 ]; then
    cargo run -- span "Successfully built LogLine v0.1.0"
    echo "Build successful! LogLine is ready to use."
else
    cargo run -- span "Build failed for LogLine v0.1.0"
    echo "Build failed."
    exit 1
fi

# Show timeline
echo "Timeline after build:"
cargo run -- timeline --limit 5

echo ""
echo "LogLine is ready to use. The binary is at: target/release/logline_seed"
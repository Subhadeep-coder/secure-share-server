#!/bin/bash

# Install Rust
curl https://sh.rustup.rs -sSf | sh -s -- -y

# Source cargo environment
export PATH="$HOME/.cargo/bin:$PATH"

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "Rust installation failed."
    exit 1
fi

# Build the project
cargo build --release

# Prepare the output directory
mkdir -p ./public
cp ./target/release/server ./public/ # Replace with your actual binary name

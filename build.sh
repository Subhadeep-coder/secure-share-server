#!/bin/bash

# Install Rust in a local directory
curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly --profile minimal --no-modify-path --default-host x86_64-unknown-linux-gnu --prefix .rustup

# Add cargo to PATH
export PATH="$PWD/.rustup/bin:$PATH"

# Build the project
cargo build --release

# Prepare the output directory
mkdir -p ./public
cp ./target/release/server ./public/ # Replace with your actual binary name

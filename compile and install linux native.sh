#!/bin/sh

# Check if the game save folder exists
if [ -d "$HOME/.local/share/love/Balatro" ]; then
  echo "Found Balatro save folder"
else
  echo "Balatro save folder not found"
  echo "Please run the game at least once before running this script"
  exit 1
fi

# Check if cargo is installed
if command -v cargo &> /dev/null; then
  echo "Found cargo"
else
  echo "Cargo not found"
  echo "Please install Rust and Cargo from https://www.rust-lang.org/tools/install"
  exit 1
fi

# Build the shared library
cargo build --release

# Compress the shared library
# if command -v upx &> /dev/null; then
#   echo "Found upx"
#   upx --best --lzma target/release/libbalalib.so
# else
#   echo "upx not found"
#   echo "Skipping compression"
# fi

# UPX breaks the library, idk why

# Copy the shared library to the save folder
cp target/release/libbalalib.so ~/.local/share/love/Balatro/balalib.so
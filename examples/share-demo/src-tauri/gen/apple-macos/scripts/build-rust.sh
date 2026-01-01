#!/bin/bash
set -e

# Source cargo environment (needed when running from Xcode)
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi

CONFIGURATION=$1
ARCHS=$2

# Determine Cargo profile
if [ "$CONFIGURATION" == "Release" ]; then
    PROFILE="release"
    CARGO_PROFILE_FLAG="--release"
else
    PROFILE="debug"
    CARGO_PROFILE_FLAG=""
fi

# Navigate to Tauri src directory
cd "${PROJECT_DIR}/../.."

# Get the first architecture (for now, build for the active arch)
ARCH=$(echo $ARCHS | awk '{print $1}')

case $ARCH in
    arm64)
        TARGET="aarch64-apple-darwin"
        ;;
    x86_64)
        TARGET="x86_64-apple-darwin"
        ;;
    *)
        echo "Unknown architecture: $ARCH"
        exit 1
        ;;
esac

echo "Building for $TARGET..."
cargo build --target $TARGET $CARGO_PROFILE_FLAG

# Find the binary (same name as the package)
BINARY_NAME=$(grep -m1 'name = ' Cargo.toml | sed 's/name = "\(.*\)"/\1/')
BINARY_PATH="target/$TARGET/$PROFILE/$BINARY_NAME"

if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Binary not found at $BINARY_PATH"
    exit 1
fi

echo "Found binary: $BINARY_PATH"

# Copy binary to Xcode's expected location
mkdir -p "${BUILT_PRODUCTS_DIR}/${EXECUTABLE_FOLDER_PATH}"
cp "$BINARY_PATH" "${BUILT_PRODUCTS_DIR}/${EXECUTABLE_PATH}"

echo "Rust build complete - binary copied to ${BUILT_PRODUCTS_DIR}/${EXECUTABLE_PATH}"

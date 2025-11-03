#!/bin/bash
#
# Intelexta Linux Launcher
#
# This script sets environment variables to work around graphics driver issues
# on some Linux systems (particularly Ubuntu with certain GPU configurations).
#
# The issues:
# - KMS: DRM_IOCTL_MODE_CREATE_DUMB failed: Permission denied
# - Failed to create GBM buffer: Permission denied
#
# The workarounds:
# - LIBGL_ALWAYS_SOFTWARE=1: Forces software rendering (Mesa/LLVMpipe)
# - WEBKIT_DISABLE_DMABUF_RENDERER=1: Disables DMA-BUF rendering in WebKit
# - WINIT_UNIX_BACKEND=x11: Forces X11 instead of Wayland
# - GDK_BACKEND=x11: Forces GTK to use X11

# Set environment variables
export LIBGL_ALWAYS_SOFTWARE=1
export WEBKIT_DISABLE_DMABUF_RENDERER=1
export WINIT_UNIX_BACKEND=x11
export GDK_BACKEND=x11

# Determine the script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Check if we're in dev mode (cargo tauri dev) or AppImage mode
if [ -f "$SCRIPT_DIR/src-tauri/Cargo.toml" ]; then
    # Development mode
    echo "Starting Intelexta in development mode..."
    cd "$SCRIPT_DIR"
    cargo tauri dev
elif [ -f "$SCRIPT_DIR/Intelexta_0.1.0_amd64_linux.AppImage" ]; then
    # AppImage mode
    echo "Starting Intelexta AppImage..."
    "$SCRIPT_DIR/Intelexta_0.1.0_amd64_linux.AppImage"
else
    echo "Error: Could not find Intelexta executable or source directory"
    echo "Please run this script from the project root or the directory containing the AppImage"
    exit 1
fi

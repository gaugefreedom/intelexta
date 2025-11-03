# Linux Graphics Issues Fix

## Problem

On some Linux systems (particularly Ubuntu with certain GPU configurations), you may encounter these errors when launching Intelexta:

```
KMS: DRM_IOCTL_MODE_CREATE_DUMB failed: Permission denied
Failed to create GBM buffer of size 1200x800: Permission denied
```

This results in a blank screen or the application failing to start.

## Root Cause

The issue is related to WebKit/GTK trying to use hardware-accelerated rendering with DMA-BUF, which may fail due to:
- Missing GPU permissions
- Incompatible graphics drivers
- Wayland/X11 compatibility issues
- GPU not supporting the required features

## Solution

### Option 1: Use the Launch Script (Recommended)

We've provided a convenience script that sets all necessary environment variables:

```bash
./launch-linux.sh
```

This script automatically:
- Detects if you're in dev mode or using an AppImage
- Sets all required environment variables
- Launches the application with the correct configuration

### Option 2: Set Environment Variables Manually

#### For Development (`cargo tauri dev`):
```bash
LIBGL_ALWAYS_SOFTWARE=1 \
WEBKIT_DISABLE_DMABUF_RENDERER=1 \
WINIT_UNIX_BACKEND=x11 \
GDK_BACKEND=x11 \
cargo tauri dev
```

#### For AppImage:
```bash
LIBGL_ALWAYS_SOFTWARE=1 \
WEBKIT_DISABLE_DMABUF_RENDERER=1 \
WINIT_UNIX_BACKEND=x11 \
GDK_BACKEND=x11 \
./Intelexta_0.1.0_amd64_linux.AppImage
```

### Option 3: Add to Shell Profile (Permanent Fix)

Add these lines to your `~/.bashrc` or `~/.zshrc`:

```bash
# Intelexta graphics workaround
export LIBGL_ALWAYS_SOFTWARE=1
export WEBKIT_DISABLE_DMABUF_RENDERER=1
export WINIT_UNIX_BACKEND=x11
export GDK_BACKEND=x11
```

Then reload your shell:
```bash
source ~/.bashrc
```

## Environment Variables Explained

| Variable | Purpose |
|----------|---------|
| `LIBGL_ALWAYS_SOFTWARE=1` | Forces software rendering (Mesa/LLVMpipe) instead of GPU acceleration |
| `WEBKIT_DISABLE_DMABUF_RENDERER=1` | Disables DMA-BUF rendering in WebKitGTK |
| `WINIT_UNIX_BACKEND=x11` | Forces Winit (Rust windowing library) to use X11 instead of Wayland |
| `GDK_BACKEND=x11` | Forces GTK to use X11 backend |

## Performance Impact

Using software rendering (`LIBGL_ALWAYS_SOFTWARE=1`) may slightly reduce graphics performance, but for a desktop application like Intelexta, the impact should be minimal and imperceptible for most users.

## Alternative Solutions

If you want to use hardware acceleration, you can try:

1. **Fix GPU permissions** (if using proprietary drivers):
   ```bash
   sudo usermod -a -G video $USER
   sudo usermod -a -G render $USER
   # Log out and log back in
   ```

2. **Update graphics drivers**:
   ```bash
   sudo ubuntu-drivers autoinstall
   ```

3. **Switch to X11** (if using Wayland):
   - Log out
   - On the login screen, click the gear icon
   - Select "Ubuntu on Xorg"
   - Log in

## Testing

To verify the fix worked:

1. Launch the application using one of the methods above
2. You should NOT see the DRM/GBM errors
3. The application window should appear normally
4. All UI elements should be visible and functional

## Additional Resources

- [Tauri Linux Troubleshooting](https://tauri.app/v1/guides/debugging/application#linux)
- [WebKitGTK Graphics Issues](https://bugs.webkit.org/show_bug.cgi?id=242251)
- [Ubuntu Graphics Driver Guide](https://help.ubuntu.com/community/BinaryDriverHowto)

#!/bin/bash
# n01d Machine Build Script
# Builds cross-platform releases using Tauri

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$SCRIPT_DIR/releases/n01d-cross-platform"

echo "╔══════════════════════════════════════════╗"
echo "║       n01d Machine Build Script          ║"
echo "╚══════════════════════════════════════════╝"

# Check dependencies
check_deps() {
    echo "[*] Checking dependencies..."
    
    if ! command -v cargo &> /dev/null; then
        echo "[-] Rust/Cargo not found. Install from https://rustup.rs"
        exit 1
    fi
    
    if ! command -v node &> /dev/null; then
        echo "[-] Node.js not found. Install from https://nodejs.org"
        exit 1
    fi
    
    echo "[+] Dependencies OK"
}

# Install Tauri CLI
install_tauri() {
    if ! command -v cargo-tauri &> /dev/null; then
        echo "[*] Installing Tauri CLI..."
        cargo install tauri-cli
    fi
    echo "[+] Tauri CLI ready"
}

# Generate icons from SVG
generate_icons() {
    echo "[*] Generating icons..."
    ICON_DIR="$PROJECT_DIR/src-tauri/icons"
    SVG_SOURCE="$SCRIPT_DIR/n01d-icon.svg"
    
    if command -v convert &> /dev/null && [ -f "$SVG_SOURCE" ]; then
        convert -background none "$SVG_SOURCE" -resize 32x32 "$ICON_DIR/32x32.png"
        convert -background none "$SVG_SOURCE" -resize 128x128 "$ICON_DIR/128x128.png"
        convert -background none "$SVG_SOURCE" -resize 256x256 "$ICON_DIR/128x128@2x.png"
        
        # Windows ICO
        convert -background none "$SVG_SOURCE" -define icon:auto-resize=256,128,64,48,32,16 "$ICON_DIR/icon.ico"
        
        # macOS ICNS (requires iconutil on mac)
        if command -v iconutil &> /dev/null; then
            mkdir -p /tmp/n01d.iconset
            for size in 16 32 64 128 256 512; do
                convert -background none "$SVG_SOURCE" -resize ${size}x${size} "/tmp/n01d.iconset/icon_${size}x${size}.png"
                convert -background none "$SVG_SOURCE" -resize $((size*2))x$((size*2)) "/tmp/n01d.iconset/icon_${size}x${size}@2x.png"
            done
            iconutil -c icns /tmp/n01d.iconset -o "$ICON_DIR/icon.icns"
            rm -rf /tmp/n01d.iconset
        fi
        
        echo "[+] Icons generated"
    else
        echo "[!] ImageMagick not found, using existing icons"
    fi
}

# Build for current platform
build() {
    echo "[*] Building n01d Machine..."
    cd "$PROJECT_DIR"
    cargo tauri build
    
    echo ""
    echo "[+] Build complete!"
    echo ""
    echo "Output files:"
    find src-tauri/target/release/bundle -type f \( -name "*.deb" -o -name "*.AppImage" -o -name "*.msi" -o -name "*.exe" -o -name "*.dmg" \) 2>/dev/null | while read f; do
        echo "  - $f"
    done
}

# Development mode
dev() {
    echo "[*] Starting development server..."
    cd "$PROJECT_DIR"
    cargo tauri dev
}

# Main
case "${1:-build}" in
    build)
        check_deps
        install_tauri
        generate_icons
        build
        ;;
    dev)
        check_deps
        install_tauri
        dev
        ;;
    icons)
        generate_icons
        ;;
    *)
        echo "Usage: $0 [build|dev|icons]"
        echo ""
        echo "Commands:"
        echo "  build  - Build release binaries (default)"
        echo "  dev    - Start development mode"
        echo "  icons  - Generate icons from SVG"
        exit 1
        ;;
esac

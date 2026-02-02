# n01d Machine

<p align="center">
  <img src="n01d-icon.svg" width="128" height="128" alt="n01d Machine Logo">
</p>

<p align="center">
  <strong>Cross-Platform Virtual Machine Manager</strong>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#download">Download</a> •
  <a href="#installation">Installation</a> •
  <a href="#usage">Usage</a> •
  <a href="#building">Building</a>
</p>

---

## Features

- 🖥️ **Manage Virtual Machines** - Create, run, and delete VMs with ease
- 💿 **ISO Management** - Browse and quick-boot ISO images
- ⚡ **Quick Boot** - Instantly boot any ISO without creating a VM
- 🎨 **Modern Dark UI** - Beautiful cyberpunk-inspired interface
- 🚀 **Hardware Acceleration** - KVM (Linux), HVF (macOS), WHPX (Windows)
- 📦 **Cross-Platform** - Works on Linux, Windows, and macOS

## Download

### Latest Release

| Platform | Download |
|----------|----------|
| 🐧 Linux (AppImage) | [n01d-machine_1.0.0_amd64.AppImage](https://github.com/bad-antics/n01d-machine/releases/latest) |
| 🐧 Linux (Debian) | [n01d-machine_1.0.0_amd64.deb](https://github.com/bad-antics/n01d-machine/releases/latest) |
| 🪟 Windows (Installer) | [n01d-machine_1.0.0_x64-setup.exe](https://github.com/bad-antics/n01d-machine/releases/latest) |
| 🪟 Windows (MSI) | [n01d-machine_1.0.0_x64.msi](https://github.com/bad-antics/n01d-machine/releases/latest) |
| 🍎 macOS (DMG) | [n01d-machine_1.0.0_x64.dmg](https://github.com/bad-antics/n01d-machine/releases/latest) |

## Prerequisites

n01d Machine requires QEMU to be installed on your system:

### Linux
\`\`\`bash
# Debian/Ubuntu
sudo apt install qemu-system-x86 qemu-utils

# Fedora
sudo dnf install qemu-system-x86 qemu-img

# Arch
sudo pacman -S qemu-full
\`\`\`

### Windows
Download QEMU for Windows from: https://qemu.weilnetz.de/w64/

Or install via Chocolatey:
\`\`\`powershell
choco install qemu
\`\`\`

### macOS
\`\`\`bash
brew install qemu
\`\`\`

## Installation

### Linux AppImage
\`\`\`bash
chmod +x n01d-machine_1.0.0_amd64.AppImage
./n01d-machine_1.0.0_amd64.AppImage
\`\`\`

### Linux Debian Package
\`\`\`bash
sudo dpkg -i n01d-machine_1.0.0_amd64.deb
\`\`\`

### Windows
Run the installer (\`.exe\` or \`.msi\`) and follow the prompts.

### macOS
Open the DMG and drag n01d Machine to your Applications folder.

## Usage

### Managing VMs

1. **Create a VM**: Click "Create VM" in the sidebar
2. **Select ISO**: Choose an ISO from the dropdown or browse
3. **Configure**: Set RAM, CPUs, and disk size
4. **Run**: Click "▶ Run" to boot from disk, "💿 Live" to boot from ISO

### Quick Boot

Click "⚡ Quick Boot ISO" in the header to instantly boot any ISO without creating a VM.

### File Locations

- **VMs**: \`~/n01d-machine/vms/\`
- **ISOs**: \`~/n01d-machine/isos/\`
- **Config**: \`~/n01d-machine/config.json\`

## Building from Source

### Prerequisites
- Rust 1.70+
- Node.js 18+
- Platform-specific dependencies (see below)

### Linux
\`\`\`bash
# Install dependencies
sudo apt install libgtk-3-dev libwebkit2gtk-4.0-dev libappindicator3-dev librsvg2-dev

# Build
cd releases/n01d-cross-platform
cargo install tauri-cli
cargo tauri build
\`\`\`

### Windows
\`\`\`powershell
cd releases\n01d-cross-platform
cargo install tauri-cli
cargo tauri build
\`\`\`

### macOS
\`\`\`bash
cd releases/n01d-cross-platform
cargo install tauri-cli
cargo tauri build
\`\`\`

## Project Structure

\`\`\`
n01d-machine/
├── n01d                    # CLI application (Python)
├── n01d-gui                # GTK GUI (Python, Linux only)
├── n01d-icon.svg           # Application icon
├── n01d-machine.desktop    # Linux desktop entry
├── releases/
│   └── n01d-cross-platform/   # Tauri cross-platform build
│       ├── public/            # Web UI
│       └── src-tauri/         # Rust backend
└── .github/
    └── workflows/
        └── release.yml     # GitHub Actions CI/CD
\`\`\`

## License

MIT License - see [LICENSE](LICENSE)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

---

<p align="center">
  Made with ❤️ by <a href="https://github.com/bad-antics">NullSec Team</a>
</p>

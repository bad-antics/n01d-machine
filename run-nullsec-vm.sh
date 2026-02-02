#!/bin/bash
#═══════════════════════════════════════════════════════════════════════
# n01d Machine - NullSec Linux VM Launcher
#═══════════════════════════════════════════════════════════════════════

CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${CYAN}"
echo " ▐ ▌   █▀█  ▄█  █▀▄    █▀▄▀█ ▄▀█ █▀▀ █ █ █ █▄ █ █▀▀"
echo " █ █   █▄█   █  █▄▀    █ ▀ █ █▀█ █▄▄ █▀█ █ █ ▀█ ██▄"
echo ""
echo "              NullSec Linux VM Console"
echo -e "${NC}"

VM_DIR="$HOME/n01d-machine/images"
VM_DISK="$VM_DIR/nullsec-linux.qcow2"
ISO_PATH="/home/antics/projects/nullsec-linux/build/nullsec-linux-1.0-amd64.iso"

# Check if ISO exists
if [ ! -f "$ISO_PATH" ]; then
    echo -e "${YELLOW}[!] NullSec ISO not found at $ISO_PATH${NC}"
    echo "    Looking for alternative..."
    ISO_PATH=$(find /home/antics -name "nullsec*.iso" -type f 2>/dev/null | head -1)
    if [ -z "$ISO_PATH" ]; then
        echo "[-] No NullSec ISO found. Please build one first."
        exit 1
    fi
    echo -e "${GREEN}[+] Found: $ISO_PATH${NC}"
fi

# Create VM disk if it doesn't exist
if [ ! -f "$VM_DISK" ]; then
    echo -e "${GREEN}[*] Creating VM disk (40GB)...${NC}"
    qemu-img create -f qcow2 "$VM_DISK" 40G
fi

# Determine mode
MODE="${1:-live}"

case "$MODE" in
    install)
        echo -e "${GREEN}[*] Starting NullSec Linux INSTALLER${NC}"
        echo "    ISO: $ISO_PATH"
        echo "    Disk: $VM_DISK"
        echo ""
        
        qemu-system-x86_64 \
            -name "n01d-NullSec-Install" \
            -enable-kvm \
            -m 4096 \
            -smp 4 \
            -cpu host \
            -boot d \
            -cdrom "$ISO_PATH" \
            -drive file="$VM_DISK",format=qcow2,if=virtio \
            -netdev user,id=net0,hostfwd=tcp::2222-:22 \
            -device virtio-net-pci,netdev=net0 \
            -vga virtio \
            -display gtk
        ;;
        
    live)
        echo -e "${GREEN}[*] Starting NullSec Linux LIVE${NC}"
        echo "    ISO: $ISO_PATH"
        echo ""
        
        qemu-system-x86_64 \
            -name "n01d-NullSec-Live" \
            -enable-kvm \
            -m 4096 \
            -smp 4 \
            -cpu host \
            -boot d \
            -cdrom "$ISO_PATH" \
            -netdev user,id=net0,hostfwd=tcp::2222-:22 \
            -device virtio-net-pci,netdev=net0 \
            -vga virtio \
            -display gtk
        ;;
        
    run)
        echo -e "${GREEN}[*] Starting installed NullSec Linux${NC}"
        echo "    Disk: $VM_DISK"
        echo ""
        
        if [ ! -f "$VM_DISK" ]; then
            echo "[-] No installed system found. Run with 'install' first."
            exit 1
        fi
        
        qemu-system-x86_64 \
            -name "n01d-NullSec" \
            -enable-kvm \
            -m 4096 \
            -smp 4 \
            -cpu host \
            -boot c \
            -drive file="$VM_DISK",format=qcow2,if=virtio \
            -netdev user,id=net0,hostfwd=tcp::2222-:22 \
            -device virtio-net-pci,netdev=net0 \
            -vga virtio \
            -display gtk
        ;;
        
    headless)
        echo -e "${GREEN}[*] Starting NullSec Linux HEADLESS${NC}"
        echo "    SSH access: ssh -p 2222 root@localhost"
        echo ""
        
        qemu-system-x86_64 \
            -name "n01d-NullSec-Headless" \
            -enable-kvm \
            -m 4096 \
            -smp 4 \
            -cpu host \
            -boot c \
            -drive file="$VM_DISK",format=qcow2,if=virtio \
            -netdev user,id=net0,hostfwd=tcp::2222-:22 \
            -device virtio-net-pci,netdev=net0 \
            -nographic
        ;;
        
    *)
        echo "Usage: $0 {live|install|run|headless}"
        echo ""
        echo "Modes:"
        echo "  live     - Boot NullSec Linux live from ISO (default)"
        echo "  install  - Install NullSec Linux to disk"
        echo "  run      - Boot from installed disk"
        echo "  headless - Run without GUI (SSH access on port 2222)"
        exit 1
        ;;
esac

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct VmConfig {
    disk: String,
    iso: Option<String>,
    ram: u32,
    cpus: u32,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct AppConfig {
    vms: HashMap<String, VmConfig>,
    default_ram: u32,
    default_cpus: u32,
}

fn get_config_dir() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join("n01d-machine")
}

fn get_config_path() -> PathBuf {
    get_config_dir().join("config.json")
}

fn load_config() -> AppConfig {
    let path = get_config_path();
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_else(|_| AppConfig {
            vms: HashMap::new(),
            default_ram: 4096,
            default_cpus: 4,
        })
    } else {
        AppConfig {
            vms: HashMap::new(),
            default_ram: 4096,
            default_cpus: 4,
        }
    }
}

fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = get_config_path();
    fs::create_dir_all(get_config_dir()).map_err(|e| e.to_string())?;
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_vms() -> HashMap<String, VmConfig> {
    load_config().vms
}

#[tauri::command]
fn get_config() -> AppConfig {
    load_config()
}

#[tauri::command]
fn list_isos() -> Vec<String> {
    let mut isos = Vec::new();
    let iso_dir = get_config_dir().join("isos");
    
    // Check n01d-machine/isos directory
    if iso_dir.exists() {
        if let Ok(entries) = fs::read_dir(&iso_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "iso").unwrap_or(false) {
                    isos.push(path.to_string_lossy().to_string());
                }
            }
        }
    }
    
    // Also check common ISO locations
    let common_paths = vec![
        dirs::home_dir().map(|h| h.join("Downloads")),
        dirs::home_dir().map(|h| h.join("ISOs")),
        dirs::home_dir().map(|h| h.join("projects")),
    ];
    
    for dir_opt in common_paths.into_iter().flatten() {
        if let Ok(entries) = fs::read_dir(&dir_opt) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "iso").unwrap_or(false) {
                    let path_str = path.to_string_lossy().to_string();
                    if !isos.contains(&path_str) {
                        isos.push(path_str);
                    }
                }
            }
        }
    }
    
    isos
}

#[tauri::command]
fn create_vm(name: String, iso: Option<String>, ram: u32, cpus: u32, disk_size: u32) -> Result<String, String> {
    let mut config = load_config();
    
    if config.vms.contains_key(&name) {
        return Err(format!("VM '{}' already exists", name));
    }
    
    let vm_dir = get_config_dir().join("vms").join(&name);
    fs::create_dir_all(&vm_dir).map_err(|e| e.to_string())?;
    
    let disk_path = vm_dir.join(format!("{}.qcow2", name));
    
    // Create disk using qemu-img
    let output = Command::new("qemu-img")
        .args(["create", "-f", "qcow2", disk_path.to_str().unwrap(), &format!("{}G", disk_size)])
        .output()
        .map_err(|e| format!("Failed to create disk: {}", e))?;
    
    if !output.status.success() {
        return Err(format!("qemu-img failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    config.vms.insert(name.clone(), VmConfig {
        disk: disk_path.to_string_lossy().to_string(),
        iso,
        ram,
        cpus,
    });
    
    save_config(&config)?;
    Ok(format!("VM '{}' created successfully", name))
}

#[tauri::command]
fn delete_vm(name: String) -> Result<String, String> {
    let mut config = load_config();
    
    if !config.vms.contains_key(&name) {
        return Err(format!("VM '{}' not found", name));
    }
    
    let vm_dir = get_config_dir().join("vms").join(&name);
    if vm_dir.exists() {
        fs::remove_dir_all(&vm_dir).map_err(|e| e.to_string())?;
    }
    
    config.vms.remove(&name);
    save_config(&config)?;
    
    Ok(format!("VM '{}' deleted", name))
}

#[tauri::command]
fn run_vm(name: String, live: bool, install: bool) -> Result<String, String> {
    let config = load_config();
    let vm = config.vms.get(&name).ok_or(format!("VM '{}' not found", name))?;
    
    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.args(["-name", &format!("n01d-{}", name)]);
    
    // Try to use KVM acceleration on Linux
    #[cfg(target_os = "linux")]
    cmd.arg("-enable-kvm");
    
    // Try to use HVF acceleration on macOS
    #[cfg(target_os = "macos")]
    cmd.args(["-accel", "hvf"]);
    
    // Try to use WHPX acceleration on Windows
    #[cfg(target_os = "windows")]
    cmd.args(["-accel", "whpx"]);
    
    cmd.args([
        "-m", &vm.ram.to_string(),
        "-smp", &vm.cpus.to_string(),
        "-cpu", "max",
        "-drive", &format!("file={},format=qcow2,if=virtio", vm.disk),
        "-netdev", "user,id=net0,hostfwd=tcp::2222-:22",
        "-device", "virtio-net-pci,netdev=net0",
        "-vga", "virtio",
        "-usb", "-device", "usb-tablet",
        "-display", "gtk",
    ]);
    
    if let Some(iso) = &vm.iso {
        if live || install {
            cmd.args(["-cdrom", iso, "-boot", "d"]);
        }
    }
    
    if !live && !install {
        cmd.args(["-boot", "c"]);
    }
    
    cmd.spawn().map_err(|e| format!("Failed to start VM: {}", e))?;
    
    Ok(format!("VM '{}' started", name))
}

#[tauri::command]
fn quick_boot_iso(iso_path: String) -> Result<String, String> {
    let config = load_config();
    
    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.args(["-name", "n01d-quickboot"]);
    
    #[cfg(target_os = "linux")]
    cmd.arg("-enable-kvm");
    
    #[cfg(target_os = "macos")]
    cmd.args(["-accel", "hvf"]);
    
    #[cfg(target_os = "windows")]
    cmd.args(["-accel", "whpx"]);
    
    cmd.args([
        "-m", &config.default_ram.to_string(),
        "-smp", &config.default_cpus.to_string(),
        "-cpu", "max",
        "-cdrom", &iso_path,
        "-boot", "d",
        "-netdev", "user,id=net0,hostfwd=tcp::2222-:22",
        "-device", "virtio-net-pci,netdev=net0",
        "-vga", "virtio",
        "-usb", "-device", "usb-tablet",
        "-display", "gtk",
    ]);
    
    cmd.spawn().map_err(|e| format!("Failed to boot ISO: {}", e))?;
    
    Ok("ISO booted".to_string())
}

#[tauri::command]
fn save_settings(default_ram: u32, default_cpus: u32) -> Result<String, String> {
    let mut config = load_config();
    config.default_ram = default_ram;
    config.default_cpus = default_cpus;
    save_config(&config)?;
    Ok("Settings saved".to_string())
}

#[tauri::command]
fn check_qemu_installed() -> bool {
    which::which("qemu-system-x86_64").is_ok()
}

fn main() {
    // Ensure config directory exists
    let config_dir = get_config_dir();
    let _ = fs::create_dir_all(config_dir.join("vms"));
    let _ = fs::create_dir_all(config_dir.join("isos"));
    
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_vms,
            get_config,
            list_isos,
            create_vm,
            delete_vm,
            run_vm,
            quick_boot_iso,
            save_settings,
            check_qemu_installed,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

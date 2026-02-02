#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod security;

use security::{SecurityManager, SecurityProfile, IsolationMode, TorConfig, VpnConfig, ProxyConfig};
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
    security_profile: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct AppConfig {
    vms: HashMap<String, VmConfig>,
    default_ram: u32,
    default_cpus: u32,
    security_profiles: HashMap<String, SecurityProfile>,
    tor_enabled: bool,
    default_security_profile: Option<String>,
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
            security_profiles: HashMap::new(),
            tor_enabled: false,
            default_security_profile: None,
        })
    } else {
        AppConfig {
            vms: HashMap::new(),
            default_ram: 4096,
            default_cpus: 4,
            security_profiles: HashMap::new(),
            tor_enabled: false,
            default_security_profile: None,
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
        security_profile: None,
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

// ==================== SECURITY COMMANDS ====================

#[tauri::command]
fn get_security_profiles() -> Vec<(String, String, SecurityProfile)> {
    SecurityManager::get_preset_profiles()
        .into_iter()
        .map(|(name, desc, profile)| (name.to_string(), desc.to_string(), profile))
        .collect()
}

#[tauri::command]
fn get_custom_security_profiles() -> HashMap<String, SecurityProfile> {
    load_config().security_profiles
}

#[tauri::command]
fn create_security_profile(name: String, profile: SecurityProfile) -> Result<String, String> {
    let mut config = load_config();
    config.security_profiles.insert(name.clone(), profile);
    save_config(&config)?;
    Ok(format!("Security profile '{}' created", name))
}

#[tauri::command]
fn delete_security_profile(name: String) -> Result<String, String> {
    let mut config = load_config();
    config.security_profiles.remove(&name);
    save_config(&config)?;
    Ok(format!("Security profile '{}' deleted", name))
}

#[tauri::command]
fn apply_security_profile(vm_name: String, profile_name: String) -> Result<String, String> {
    let mut config = load_config();
    if let Some(vm) = config.vms.get_mut(&vm_name) {
        vm.security_profile = Some(profile_name.clone());
        save_config(&config)?;
        Ok(format!("Applied '{}' profile to VM '{}'", profile_name, vm_name))
    } else {
        Err(format!("VM '{}' not found", vm_name))
    }
}

#[tauri::command]
fn check_tor_installed() -> bool {
    which::which("tor").is_ok()
}

#[tauri::command]
fn check_tor_running() -> bool {
    security::check_tor_status()
}

#[tauri::command]
fn start_tor_service() -> Result<String, String> {
    #[cfg(target_os = "linux")]
    {
        Command::new("systemctl")
            .args(["start", "tor"])
            .output()
            .map_err(|e| format!("Failed to start Tor: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        Command::new("brew")
            .args(["services", "start", "tor"])
            .output()
            .map_err(|e| format!("Failed to start Tor: {}", e))?;
    }
    
    #[cfg(target_os = "windows")]
    {
        Command::new("tor")
            .spawn()
            .map_err(|e| format!("Failed to start Tor: {}", e))?;
    }
    
    Ok("Tor service started".to_string())
}

#[tauri::command]
fn stop_tor_service() -> Result<String, String> {
    #[cfg(target_os = "linux")]
    {
        Command::new("systemctl")
            .args(["stop", "tor"])
            .output()
            .map_err(|e| format!("Failed to stop Tor: {}", e))?;
    }
    
    #[cfg(target_os = "macos")]
    {
        Command::new("brew")
            .args(["services", "stop", "tor"])
            .output()
            .map_err(|e| format!("Failed to stop Tor: {}", e))?;
    }
    
    #[cfg(target_os = "windows")]
    {
        Command::new("taskkill")
            .args(["/IM", "tor.exe", "/F"])
            .output()
            .map_err(|e| format!("Failed to stop Tor: {}", e))?;
    }
    
    Ok("Tor service stopped".to_string())
}

#[tauri::command]
fn new_tor_identity() -> Result<String, String> {
    security::new_tor_circuit(9051)?;
    Ok("New Tor identity requested".to_string())
}

#[tauri::command]
fn check_vpn_installed() -> HashMap<String, bool> {
    let mut results = HashMap::new();
    results.insert("openvpn".to_string(), which::which("openvpn").is_ok());
    results.insert("wireguard".to_string(), which::which("wg").is_ok() || which::which("wg-quick").is_ok());
    results
}

#[tauri::command]
fn run_vm_secure(name: String, profile_name: String, live: bool, install: bool) -> Result<String, String> {
    let config = load_config();
    let vm = config.vms.get(&name).ok_or(format!("VM '{}' not found", name))?;
    
    // Get security profile
    let security_profile = if let Some(profile) = config.security_profiles.get(&profile_name) {
        profile.clone()
    } else {
        // Check preset profiles
        SecurityManager::get_preset_profiles()
            .into_iter()
            .find(|(n, _, _)| *n == profile_name)
            .map(|(_, _, p)| p)
            .ok_or(format!("Security profile '{}' not found", profile_name))?
    };
    
    let security_manager = SecurityManager::new(get_config_dir());
    let security_args = security_manager.generate_qemu_security_args(&security_profile);
    
    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.args(["-name", &format!("n01d-{}", name)]);
    
    #[cfg(target_os = "linux")]
    cmd.arg("-enable-kvm");
    
    #[cfg(target_os = "macos")]
    cmd.args(["-accel", "hvf"]);
    
    #[cfg(target_os = "windows")]
    cmd.args(["-accel", "whpx"]);
    
    cmd.args([
        "-m", &vm.ram.to_string(),
        "-smp", &vm.cpus.to_string(),
        "-cpu", "max",
        "-drive", &format!("file={},format=qcow2,if=virtio", vm.disk),
        "-vga", "virtio",
        "-usb", "-device", "usb-tablet",
        "-display", "gtk",
    ]);
    
    // Add security arguments
    for arg in security_args {
        cmd.arg(arg);
    }
    
    if let Some(iso) = &vm.iso {
        if live || install {
            cmd.args(["-cdrom", iso, "-boot", "d"]);
        }
    }
    
    if !live && !install {
        cmd.args(["-boot", "c"]);
    }
    
    cmd.spawn().map_err(|e| format!("Failed to start VM: {}", e))?;
    
    Ok(format!("VM '{}' started with '{}' security profile", name, profile_name))
}

fn main() {
    // Ensure config directory exists
    let config_dir = get_config_dir();
    let _ = fs::create_dir_all(config_dir.join("vms"));
    let _ = fs::create_dir_all(config_dir.join("isos"));
    let _ = fs::create_dir_all(config_dir.join("security"));
    let _ = fs::create_dir_all(config_dir.join("tor"));
    let _ = fs::create_dir_all(config_dir.join("vpn"));
    
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
            // Security commands
            get_security_profiles,
            get_custom_security_profiles,
            create_security_profile,
            delete_security_profile,
            apply_security_profile,
            check_tor_installed,
            check_tor_running,
            start_tor_service,
            stop_tor_service,
            new_tor_identity,
            check_vpn_installed,
            run_vm_secure,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

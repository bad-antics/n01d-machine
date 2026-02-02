//! VM Management Module

use std::path::PathBuf;
use std::process::Command;
use std::fs;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    pub name: String,
    pub ram: String,
    pub disk: String,
    pub cpus: u32,
    pub iso: Option<PathBuf>,
    pub template: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmInfo {
    pub name: String,
    pub status: VmStatus,
    pub ram: String,
    pub cpus: u32,
    pub disk_path: PathBuf,
    pub snapshots: Vec<String>,
    pub network: String,
    pub isolated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VmStatus {
    Running,
    Stopped,
    Paused,
    Creating,
    Error(String),
}

impl std::fmt::Display for VmStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmStatus::Running => write!(f, "Running"),
            VmStatus::Stopped => write!(f, "Stopped"),
            VmStatus::Paused => write!(f, "Paused"),
            VmStatus::Creating => write!(f, "Creating"),
            VmStatus::Error(e) => write!(f, "Error: {}", e),
        }
    }
}

fn get_vm_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("NullSec-VMs")
}

pub fn list_vms(verbose: bool) -> Result<()> {
    use colored::*;
    
    let vm_dir = get_vm_dir();
    
    if !vm_dir.exists() {
        println!("{} No VMs found. VM directory: {}", "[!]".yellow(), vm_dir.display());
        return Ok(());
    }
    
    println!("{}", "═".repeat(60).blue());
    println!("{:^60}", "NullSec VMs".bold());
    println!("{}", "═".repeat(60).blue());
    
    let mut found = false;
    for entry in fs::read_dir(&vm_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            let config_path = path.join("vm.toml");
            if config_path.exists() {
                found = true;
                let config_str = fs::read_to_string(&config_path)?;
                let info: VmInfo = toml::from_str(&config_str).unwrap_or_else(|_| VmInfo {
                    name: path.file_name().unwrap().to_string_lossy().to_string(),
                    status: VmStatus::Stopped,
                    ram: "unknown".into(),
                    cpus: 0,
                    disk_path: path.clone(),
                    snapshots: vec![],
                    network: "unknown".into(),
                    isolated: false,
                });
                
                let status_color = match info.status {
                    VmStatus::Running => "Running".green(),
                    VmStatus::Stopped => "Stopped".red(),
                    VmStatus::Paused => "Paused".yellow(),
                    _ => info.status.to_string().normal(),
                };
                
                println!("\n{} {}", "▶".cyan(), info.name.bold());
                println!("  Status: {}", status_color);
                println!("  RAM: {} | CPUs: {}", info.ram, info.cpus);
                
                if verbose {
                    println!("  Disk: {}", info.disk_path.display());
                    println!("  Network: {} | Isolated: {}", info.network, info.isolated);
                    if !info.snapshots.is_empty() {
                        println!("  Snapshots: {}", info.snapshots.join(", "));
                    }
                }
            }
        }
    }
    
    if !found {
        println!("{} No VMs found", "[!]".yellow());
    }
    
    println!("{}", "═".repeat(60).blue());
    Ok(())
}

pub fn create_vm(config: VmConfig) -> Result<()> {
    let vm_dir = get_vm_dir().join(&config.name);
    
    // Create VM directory
    fs::create_dir_all(&vm_dir)?;
    
    // Parse disk size
    let disk_size = &config.disk;
    let disk_path = vm_dir.join(format!("{}.qcow2", config.name));
    
    // Create virtual disk
    let output = Command::new("qemu-img")
        .args(["create", "-f", "qcow2"])
        .arg(&disk_path)
        .arg(disk_size)
        .output()
        .context("Failed to create virtual disk")?;
    
    if !output.status.success() {
        anyhow::bail!("qemu-img failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    // Save VM config
    let info = VmInfo {
        name: config.name.clone(),
        status: VmStatus::Stopped,
        ram: config.ram,
        cpus: config.cpus,
        disk_path,
        snapshots: vec![],
        network: "nat".into(),
        isolated: false,
    };
    
    let config_path = vm_dir.join("vm.toml");
    let config_str = toml::to_string_pretty(&info)?;
    fs::write(&config_path, config_str)?;
    
    // Create launcher script
    create_launcher_script(&vm_dir, &info, config.iso.as_ref())?;
    
    Ok(())
}

fn create_launcher_script(vm_dir: &PathBuf, info: &VmInfo, iso: Option<&PathBuf>) -> Result<()> {
    let script_path = vm_dir.join("start.sh");
    
    let iso_arg = iso.map(|p| format!("-cdrom {} -boot d", p.display()))
        .unwrap_or_default();
    
    let script = format!(r#"#!/bin/bash
# NullSec VM Launcher - {}

VM_DIR="$(dirname "$0")"
DISK="$VM_DIR/{}.qcow2"

qemu-system-x86_64 \
    -m {} \
    -smp {} \
    -cpu host \
    -enable-kvm \
    -drive file="$DISK",format=qcow2 \
    {} \
    -display gtk \
    -name "{}" \
    "$@"
"#, info.name, info.name, info.ram, info.cpus, iso_arg, info.name);
    
    fs::write(&script_path, script)?;
    
    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms)?;
    }
    
    Ok(())
}

pub fn start_vm(name: &str, isolated: bool, network: &str, headless: bool) -> Result<()> {
    let vm_dir = get_vm_dir().join(name);
    let config_path = vm_dir.join("vm.toml");
    
    if !config_path.exists() {
        anyhow::bail!("VM '{}' not found", name);
    }
    
    let config_str = fs::read_to_string(&config_path)?;
    let mut info: VmInfo = toml::from_str(&config_str)?;
    
    // Build QEMU command
    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.args(["-m", &info.ram]);
    cmd.args(["-smp", &info.cpus.to_string()]);
    cmd.args(["-cpu", "host"]);
    cmd.arg("-enable-kvm");
    cmd.args(["-drive", &format!("file={},format=qcow2", info.disk_path.display())]);
    cmd.args(["-name", name]);
    
    // Network configuration
    match network {
        "none" => {
            cmd.args(["-nic", "none"]);
        }
        "isolated" => {
            cmd.args(["-nic", "user,restrict=yes"]);
        }
        "nat" => {
            cmd.args(["-nic", "user"]);
        }
        "bridge" => {
            cmd.args(["-nic", "bridge,br=br0"]);
        }
        _ => {
            cmd.args(["-nic", "user"]);
        }
    }
    
    // Display
    if headless {
        cmd.args(["-display", "none"]);
        cmd.arg("-daemonize");
    } else {
        cmd.args(["-display", "gtk"]);
    }
    
    // Apply isolation if requested
    if isolated {
        // We'll handle this through sandbox module
        println!("Applying isolation settings...");
    }
    
    // Start VM
    let child = cmd.spawn().context("Failed to start VM")?;
    
    // Update status
    info.status = VmStatus::Running;
    info.network = network.to_string();
    info.isolated = isolated;
    
    let config_str = toml::to_string_pretty(&info)?;
    fs::write(&config_path, config_str)?;
    
    // Save PID
    let pid_path = vm_dir.join("vm.pid");
    fs::write(&pid_path, child.id().to_string())?;
    
    Ok(())
}

pub fn stop_vm(name: &str, force: bool) -> Result<()> {
    let vm_dir = get_vm_dir().join(name);
    let pid_path = vm_dir.join("vm.pid");
    let config_path = vm_dir.join("vm.toml");
    
    if pid_path.exists() {
        let pid_str = fs::read_to_string(&pid_path)?;
        let pid: i32 = pid_str.trim().parse()?;
        
        // Send signal
        #[cfg(unix)]
        {
            use nix::sys::signal::{self, Signal};
            use nix::unistd::Pid;
            
            let sig = if force { Signal::SIGKILL } else { Signal::SIGTERM };
            let _ = signal::kill(Pid::from_raw(pid), sig);
        }
        
        fs::remove_file(&pid_path)?;
    }
    
    // Update status
    if config_path.exists() {
        let config_str = fs::read_to_string(&config_path)?;
        let mut info: VmInfo = toml::from_str(&config_str)?;
        info.status = VmStatus::Stopped;
        let config_str = toml::to_string_pretty(&info)?;
        fs::write(&config_path, config_str)?;
    }
    
    Ok(())
}

pub fn create_snapshot(vm: &str, name: &str) -> Result<()> {
    let vm_dir = get_vm_dir().join(vm);
    let config_path = vm_dir.join("vm.toml");
    
    if !config_path.exists() {
        anyhow::bail!("VM '{}' not found", vm);
    }
    
    let config_str = fs::read_to_string(&config_path)?;
    let mut info: VmInfo = toml::from_str(&config_str)?;
    
    // Create snapshot with qemu-img
    let output = Command::new("qemu-img")
        .args(["snapshot", "-c", name])
        .arg(&info.disk_path)
        .output()?;
    
    if !output.status.success() {
        anyhow::bail!("Failed to create snapshot: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    // Update config
    info.snapshots.push(name.to_string());
    let config_str = toml::to_string_pretty(&info)?;
    fs::write(&config_path, config_str)?;
    
    Ok(())
}

pub fn restore_snapshot(vm: &str, snapshot: &str) -> Result<()> {
    let vm_dir = get_vm_dir().join(vm);
    let config_path = vm_dir.join("vm.toml");
    
    if !config_path.exists() {
        anyhow::bail!("VM '{}' not found", vm);
    }
    
    let config_str = fs::read_to_string(&config_path)?;
    let info: VmInfo = toml::from_str(&config_str)?;
    
    // Restore snapshot with qemu-img
    let output = Command::new("qemu-img")
        .args(["snapshot", "-a", snapshot])
        .arg(&info.disk_path)
        .output()?;
    
    if !output.status.success() {
        anyhow::bail!("Failed to restore snapshot: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    Ok(())
}

pub fn show_config() -> Result<()> {
    use colored::*;
    
    let config_path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("nullsec-vm")
        .join("config.toml");
    
    if config_path.exists() {
        let config = fs::read_to_string(&config_path)?;
        println!("{}", "Current Configuration:".green().bold());
        println!("{}", config);
    } else {
        println!("{}", "No configuration file found. Using defaults.".yellow());
        println!("\nDefault settings:");
        println!("  VM Directory: ~/NullSec-VMs");
        println!("  Default RAM: 2G");
        println!("  Default CPUs: 2");
        println!("  Default Disk: 20G");
        println!("  Default Isolation: medium");
    }
    
    Ok(())
}

pub fn set_config(value: &str) -> Result<()> {
    // Parse key=value
    let parts: Vec<&str> = value.splitn(2, '=').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid config format. Use: key=value");
    }
    
    let _key = parts[0];
    let _val = parts[1];
    
    // TODO: Implement config setting
    println!("Config setting not yet implemented");
    
    Ok(())
}

//! Network Module - Virtual network management

use anyhow::{Result, Context};
use std::path::Path;
use std::process::Command;
use std::fs;

#[derive(Debug, Clone)]
pub struct VirtualNetwork {
    pub name: String,
    pub mode: NetworkMode,
    pub subnet: Option<String>,
    pub gateway: Option<String>,
    pub dns: Vec<String>,
    pub bridge: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NetworkMode {
    Nat,
    Isolated,
    Bridge,
    Host,
}

impl std::str::FromStr for NetworkMode {
    type Err = anyhow::Error;
    
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "nat" => Ok(NetworkMode::Nat),
            "isolated" | "none" => Ok(NetworkMode::Isolated),
            "bridge" | "bridged" => Ok(NetworkMode::Bridge),
            "host" => Ok(NetworkMode::Host),
            _ => anyhow::bail!("Unknown network mode: {}", s),
        }
    }
}

pub fn list_networks() -> Result<()> {
    use colored::*;
    
    println!("{}", "═".repeat(60).blue());
    println!("{:^60}", "NullSec Virtual Networks".bold());
    println!("{}", "═".repeat(60).blue());
    
    // List existing bridges
    println!("\n{}", "System Bridges:".green().bold());
    let output = Command::new("ip")
        .args(["link", "show", "type", "bridge"])
        .output()
        .context("Failed to list bridges")?;
    
    if output.status.success() {
        let bridges = String::from_utf8_lossy(&output.stdout);
        if bridges.trim().is_empty() {
            println!("  No bridges found");
        } else {
            for line in bridges.lines() {
                if line.contains("mtu") {
                    println!("  {}", line);
                }
            }
        }
    }
    
    // List tap devices
    println!("\n{}", "TAP Devices:".green().bold());
    let output = Command::new("ip")
        .args(["link", "show", "type", "tun"])
        .output()
        .context("Failed to list TAP devices")?;
    
    if output.status.success() {
        let taps = String::from_utf8_lossy(&output.stdout);
        if taps.trim().is_empty() {
            println!("  No TAP devices found");
        } else {
            for line in taps.lines() {
                if line.contains("mtu") {
                    println!("  {}", line);
                }
            }
        }
    }
    
    // Show iptables NAT rules
    println!("\n{}", "NAT Rules:".green().bold());
    let output = Command::new("sudo")
        .args(["iptables", "-t", "nat", "-L", "POSTROUTING", "-n"])
        .output();
    
    if let Ok(output) = output {
        if output.status.success() {
            let rules = String::from_utf8_lossy(&output.stdout);
            for line in rules.lines().skip(2) {
                if !line.trim().is_empty() {
                    println!("  {}", line);
                }
            }
        }
    }
    
    println!("{}", "═".repeat(60).blue());
    
    Ok(())
}

pub fn create_network(name: &str, mode: &str, subnet: Option<&str>) -> Result<()> {
    use colored::*;
    
    let network_mode: NetworkMode = mode.parse()?;
    
    println!("{} Creating network '{}' in {} mode", "[*]".blue(), name, mode);
    
    match network_mode {
        NetworkMode::Bridge => {
            // Create bridge interface
            let bridge_name = format!("nullsec-{}", name);
            
            // Create bridge
            let status = Command::new("sudo")
                .args(["ip", "link", "add", &bridge_name, "type", "bridge"])
                .status()
                .context("Failed to create bridge")?;
            
            if !status.success() {
                println!("{} Bridge may already exist, continuing...", "[!]".yellow());
            }
            
            // Set bridge up
            Command::new("sudo")
                .args(["ip", "link", "set", &bridge_name, "up"])
                .status()?;
            
            // Assign IP if subnet provided
            if let Some(subnet) = subnet {
                // Parse subnet and assign gateway IP
                let gateway = subnet.replace(".0/", ".1/");
                Command::new("sudo")
                    .args(["ip", "addr", "add", &gateway, "dev", &bridge_name])
                    .status()?;
            }
            
            println!("{} Bridge '{}' created", "[+]".green(), bridge_name);
        }
        
        NetworkMode::Nat => {
            // Create bridge with NAT
            let bridge_name = format!("nullsec-{}", name);
            let subnet = subnet.unwrap_or("10.10.0.0/24");
            let gateway = subnet.replace(".0/", ".1/").split('/').next().unwrap().to_string();
            
            // Create bridge
            let _ = Command::new("sudo")
                .args(["ip", "link", "add", &bridge_name, "type", "bridge"])
                .status();
            
            // Set bridge up
            Command::new("sudo")
                .args(["ip", "link", "set", &bridge_name, "up"])
                .status()?;
            
            // Assign IP
            let _ = Command::new("sudo")
                .args(["ip", "addr", "add", &format!("{}/24", gateway), "dev", &bridge_name])
                .status();
            
            // Enable IP forwarding
            Command::new("sudo")
                .args(["sysctl", "-w", "net.ipv4.ip_forward=1"])
                .status()?;
            
            // Add NAT rule
            let _ = Command::new("sudo")
                .args([
                    "iptables", "-t", "nat", "-A", "POSTROUTING",
                    "-s", subnet, "-j", "MASQUERADE"
                ])
                .status();
            
            println!("{} NAT network '{}' created with gateway {}", "[+]".green(), name, gateway);
        }
        
        NetworkMode::Isolated => {
            // Create isolated network (no routing)
            let bridge_name = format!("nullsec-{}", name);
            
            let _ = Command::new("sudo")
                .args(["ip", "link", "add", &bridge_name, "type", "bridge"])
                .status();
            
            Command::new("sudo")
                .args(["ip", "link", "set", &bridge_name, "up"])
                .status()?;
            
            // Block all forwarding for this bridge
            let _ = Command::new("sudo")
                .args([
                    "iptables", "-I", "FORWARD", "-i", &bridge_name, "-j", "DROP"
                ])
                .status();
            
            println!("{} Isolated network '{}' created (no external access)", "[+]".green(), name);
        }
        
        NetworkMode::Host => {
            println!("{} Host mode doesn't require network creation", "[*]".blue());
        }
    }
    
    Ok(())
}

pub fn delete_network(name: &str) -> Result<()> {
    use colored::*;
    
    let bridge_name = format!("nullsec-{}", name);
    
    // Bring down bridge
    let _ = Command::new("sudo")
        .args(["ip", "link", "set", &bridge_name, "down"])
        .status();
    
    // Delete bridge
    let status = Command::new("sudo")
        .args(["ip", "link", "delete", &bridge_name])
        .status()
        .context("Failed to delete bridge")?;
    
    if status.success() {
        println!("{} Network '{}' deleted", "[+]".green(), name);
    } else {
        println!("{} Failed to delete network '{}'", "[-]".red(), name);
    }
    
    Ok(())
}

pub fn inspect_traffic(target: &str, output: Option<&Path>) -> Result<()> {
    use colored::*;
    
    println!("{} Starting traffic capture for '{}'", "[*]".blue(), target);
    println!("{} Press Ctrl+C to stop capture", "[!]".yellow());
    
    let mut cmd = Command::new("sudo");
    cmd.args(["tcpdump", "-i"]);
    
    // Determine interface
    let interface = if target.starts_with("nullsec-") {
        target.to_string()
    } else {
        format!("nullsec-{}", target)
    };
    
    cmd.arg(&interface);
    cmd.args(["-n", "-v"]);
    
    // Output to file if specified
    if let Some(path) = output {
        cmd.args(["-w", &path.to_string_lossy()]);
        println!("{} Writing to: {}", "[*]".blue(), path.display());
    }
    
    // Run capture
    let status = cmd.status().context("Failed to start tcpdump")?;
    
    if !status.success() {
        println!("{} Capture ended or interface not found", "[-]".red());
    }
    
    Ok(())
}

/// Create a TAP device for a VM
pub fn create_tap_device(name: &str, bridge: &str) -> Result<String> {
    let tap_name = format!("tap-{}", name);
    
    // Create TAP device
    Command::new("sudo")
        .args(["ip", "tuntap", "add", &tap_name, "mode", "tap"])
        .status()
        .context("Failed to create TAP device")?;
    
    // Set TAP up
    Command::new("sudo")
        .args(["ip", "link", "set", &tap_name, "up"])
        .status()?;
    
    // Add to bridge
    Command::new("sudo")
        .args(["ip", "link", "set", &tap_name, "master", bridge])
        .status()?;
    
    Ok(tap_name)
}

/// Delete a TAP device
pub fn delete_tap_device(name: &str) -> Result<()> {
    Command::new("sudo")
        .args(["ip", "tuntap", "delete", name, "mode", "tap"])
        .status()
        .context("Failed to delete TAP device")?;
    
    Ok(())
}

/// VPN Integration
pub mod vpn {
    use super::*;
    
    #[derive(Debug)]
    pub struct VpnConfig {
        pub name: String,
        pub provider: VpnProvider,
        pub config_file: Option<String>,
        pub credentials: Option<(String, String)>,
    }
    
    #[derive(Debug, Clone)]
    pub enum VpnProvider {
        OpenVPN,
        WireGuard,
        Custom,
    }
    
    pub fn connect_openvpn(config_file: &str) -> Result<()> {
        use colored::*;
        
        println!("{} Connecting via OpenVPN...", "[*]".blue());
        
        Command::new("sudo")
            .args(["openvpn", "--config", config_file, "--daemon"])
            .status()
            .context("Failed to start OpenVPN")?;
        
        println!("{} OpenVPN connection started", "[+]".green());
        Ok(())
    }
    
    pub fn connect_wireguard(interface: &str, config_file: &str) -> Result<()> {
        use colored::*;
        
        println!("{} Bringing up WireGuard interface '{}'...", "[*]".blue(), interface);
        
        // Copy config
        let target_path = format!("/etc/wireguard/{}.conf", interface);
        Command::new("sudo")
            .args(["cp", config_file, &target_path])
            .status()?;
        
        // Bring up interface
        Command::new("sudo")
            .args(["wg-quick", "up", interface])
            .status()
            .context("Failed to bring up WireGuard")?;
        
        println!("{} WireGuard connection established", "[+]".green());
        Ok(())
    }
    
    pub fn disconnect_vpn(provider: VpnProvider, interface: Option<&str>) -> Result<()> {
        use colored::*;
        
        match provider {
            VpnProvider::OpenVPN => {
                Command::new("sudo")
                    .args(["killall", "openvpn"])
                    .status()?;
            }
            VpnProvider::WireGuard => {
                if let Some(iface) = interface {
                    Command::new("sudo")
                        .args(["wg-quick", "down", iface])
                        .status()?;
                }
            }
            VpnProvider::Custom => {}
        }
        
        println!("{} VPN disconnected", "[+]".green());
        Ok(())
    }
}

/// Proxy Support
pub mod proxy {
    use super::*;
    
    #[derive(Debug)]
    pub struct ProxyConfig {
        pub proxy_type: ProxyType,
        pub host: String,
        pub port: u16,
        pub auth: Option<(String, String)>,
    }
    
    #[derive(Debug, Clone)]
    pub enum ProxyType {
        Http,
        Https,
        Socks4,
        Socks5,
    }
    
    pub fn start_tor_proxy() -> Result<()> {
        use colored::*;
        
        println!("{} Starting Tor proxy...", "[*]".blue());
        
        Command::new("tor")
            .args(["--runasdaemon", "1"])
            .status()
            .context("Failed to start Tor")?;
        
        println!("{} Tor proxy running on 127.0.0.1:9050", "[+]".green());
        Ok(())
    }
    
    pub fn setup_transparent_proxy(port: u16) -> Result<()> {
        use colored::*;
        
        println!("{} Setting up transparent proxy on port {}", "[*]".blue(), port);
        
        // Add iptables rules for transparent proxying
        Command::new("sudo")
            .args([
                "iptables", "-t", "nat", "-A", "OUTPUT",
                "-p", "tcp", "--dport", "80",
                "-j", "REDIRECT", "--to-port", &port.to_string()
            ])
            .status()?;
        
        Command::new("sudo")
            .args([
                "iptables", "-t", "nat", "-A", "OUTPUT",
                "-p", "tcp", "--dport", "443",
                "-j", "REDIRECT", "--to-port", &port.to_string()
            ])
            .status()?;
        
        println!("{} Transparent proxy configured", "[+]".green());
        Ok(())
    }
}

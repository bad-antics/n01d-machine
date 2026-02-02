// Security Module for n01d Machine
// Provides sandboxing, network isolation, VPN, Tor, and proxy support

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Security profile for a VM
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SecurityProfile {
    pub name: String,
    pub sandbox_enabled: bool,
    pub network_isolation: NetworkIsolation,
    pub tor_enabled: bool,
    pub vpn_config: Option<VpnConfig>,
    pub proxy_config: Option<ProxyConfig>,
    pub firewall_rules: Vec<FirewallRule>,
    pub virtual_devices: Vec<VirtualDevice>,
}

/// Network isolation modes
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct NetworkIsolation {
    pub mode: IsolationMode,
    pub allow_host_access: bool,
    pub allow_internet: bool,
    pub isolated_network_id: Option<String>,
    pub mac_address: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub enum IsolationMode {
    #[default]
    None,
    /// Complete network isolation - no external access
    Full,
    /// NAT with host-only access
    HostOnly,
    /// Internal network between VMs only
    Internal,
    /// Bridged but filtered
    Filtered,
    /// Route all traffic through Tor
    TorOnly,
    /// Route through VPN
    VpnOnly,
}

/// VPN Configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VpnConfig {
    pub provider: VpnProvider,
    pub config_file: Option<String>,
    pub server: Option<String>,
    pub port: u16,
    pub protocol: VpnProtocol,
    pub username: Option<String>,
    pub kill_switch: bool,
    pub dns_leak_protection: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum VpnProvider {
    #[default]
    OpenVPN,
    WireGuard,
    Custom,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum VpnProtocol {
    #[default]
    UDP,
    TCP,
}

/// Proxy Configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProxyConfig {
    pub proxy_type: ProxyType,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub chain: Vec<ProxyChainEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProxyChainEntry {
    pub proxy_type: ProxyType,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum ProxyType {
    #[default]
    Socks5,
    Socks4,
    Http,
    Https,
}

/// Firewall rule
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FirewallRule {
    pub action: FirewallAction,
    pub direction: TrafficDirection,
    pub protocol: Option<String>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub port: Option<u16>,
    pub port_range: Option<(u16, u16)>,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FirewallAction {
    Allow,
    Deny,
    Drop,
    Log,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TrafficDirection {
    Inbound,
    Outbound,
    Both,
}

/// Virtual Device for sandboxing
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VirtualDevice {
    pub device_type: VirtualDeviceType,
    pub name: String,
    pub enabled: bool,
    pub passthrough: bool,
    pub isolated: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum VirtualDeviceType {
    NetworkAdapter,
    UsbController,
    StorageController,
    AudioDevice,
    SerialPort,
    Tpm,
}

/// Tor configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TorConfig {
    pub socks_port: u16,
    pub control_port: u16,
    pub dns_port: u16,
    pub transparent_proxy: bool,
    pub bridge_enabled: bool,
    pub bridges: Vec<String>,
    pub exit_nodes: Option<Vec<String>>,
    pub exclude_exit_nodes: Option<Vec<String>>,
    pub strict_nodes: bool,
    pub new_circuit_period: u32,
}

impl Default for TorConfig {
    fn default() -> Self {
        TorConfig {
            socks_port: 9050,
            control_port: 9051,
            dns_port: 5353,
            transparent_proxy: true,
            bridge_enabled: false,
            bridges: Vec::new(),
            exit_nodes: None,
            exclude_exit_nodes: None,
            strict_nodes: false,
            new_circuit_period: 30,
        }
    }
}

/// Security Manager
pub struct SecurityManager {
    config_dir: PathBuf,
    profiles: HashMap<String, SecurityProfile>,
    tor_config: TorConfig,
}

impl SecurityManager {
    pub fn new(config_dir: PathBuf) -> Self {
        let profiles_path = config_dir.join("security_profiles.json");
        let profiles = if profiles_path.exists() {
            let content = fs::read_to_string(&profiles_path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        };

        SecurityManager {
            config_dir,
            profiles,
            tor_config: TorConfig::default(),
        }
    }

    pub fn save_profiles(&self) -> Result<(), String> {
        let path = self.config_dir.join("security_profiles.json");
        let content = serde_json::to_string_pretty(&self.profiles)
            .map_err(|e| e.to_string())?;
        fs::write(path, content).map_err(|e| e.to_string())
    }

    /// Create a new security profile
    pub fn create_profile(&mut self, name: String) -> SecurityProfile {
        let profile = SecurityProfile {
            name: name.clone(),
            sandbox_enabled: true,
            network_isolation: NetworkIsolation::default(),
            tor_enabled: false,
            vpn_config: None,
            proxy_config: None,
            firewall_rules: Self::default_firewall_rules(),
            virtual_devices: Self::default_virtual_devices(),
        };
        self.profiles.insert(name, profile.clone());
        let _ = self.save_profiles();
        profile
    }

    /// Get preset security profiles
    pub fn get_preset_profiles() -> Vec<(&'static str, &'static str, SecurityProfile)> {
        vec![
            (
                "paranoid",
                "Maximum security - Full isolation, Tor routing, no host access",
                SecurityProfile {
                    name: "paranoid".to_string(),
                    sandbox_enabled: true,
                    network_isolation: NetworkIsolation {
                        mode: IsolationMode::TorOnly,
                        allow_host_access: false,
                        allow_internet: true,
                        isolated_network_id: None,
                        mac_address: Some("52:54:00:00:00:01".to_string()),
                    },
                    tor_enabled: true,
                    vpn_config: None,
                    proxy_config: None,
                    firewall_rules: vec![
                        FirewallRule {
                            action: FirewallAction::Deny,
                            direction: TrafficDirection::Outbound,
                            protocol: Some("icmp".to_string()),
                            source: None,
                            destination: None,
                            port: None,
                            port_range: None,
                            description: "Block ICMP to prevent fingerprinting".to_string(),
                        },
                        FirewallRule {
                            action: FirewallAction::Allow,
                            direction: TrafficDirection::Outbound,
                            protocol: Some("tcp".to_string()),
                            source: None,
                            destination: Some("127.0.0.1".to_string()),
                            port: Some(9050),
                            port_range: None,
                            description: "Allow Tor SOCKS".to_string(),
                        },
                    ],
                    virtual_devices: vec![
                        VirtualDevice {
                            device_type: VirtualDeviceType::NetworkAdapter,
                            name: "tor-net".to_string(),
                            enabled: true,
                            passthrough: false,
                            isolated: true,
                        },
                    ],
                },
            ),
            (
                "stealth",
                "VPN + Tor chain for maximum anonymity",
                SecurityProfile {
                    name: "stealth".to_string(),
                    sandbox_enabled: true,
                    network_isolation: NetworkIsolation {
                        mode: IsolationMode::VpnOnly,
                        allow_host_access: false,
                        allow_internet: true,
                        isolated_network_id: None,
                        mac_address: Some("52:54:00:00:00:02".to_string()),
                    },
                    tor_enabled: true,
                    vpn_config: Some(VpnConfig {
                        provider: VpnProvider::WireGuard,
                        config_file: None,
                        server: None,
                        port: 51820,
                        protocol: VpnProtocol::UDP,
                        username: None,
                        kill_switch: true,
                        dns_leak_protection: true,
                    }),
                    proxy_config: None,
                    firewall_rules: Self::default_firewall_rules(),
                    virtual_devices: Self::default_virtual_devices(),
                },
            ),
            (
                "isolated",
                "Complete network isolation - no internet access",
                SecurityProfile {
                    name: "isolated".to_string(),
                    sandbox_enabled: true,
                    network_isolation: NetworkIsolation {
                        mode: IsolationMode::Full,
                        allow_host_access: false,
                        allow_internet: false,
                        isolated_network_id: Some("isolated-net-1".to_string()),
                        mac_address: None,
                    },
                    tor_enabled: false,
                    vpn_config: None,
                    proxy_config: None,
                    firewall_rules: vec![
                        FirewallRule {
                            action: FirewallAction::Deny,
                            direction: TrafficDirection::Both,
                            protocol: None,
                            source: None,
                            destination: None,
                            port: None,
                            port_range: None,
                            description: "Block all traffic".to_string(),
                        },
                    ],
                    virtual_devices: Vec::new(),
                },
            ),
            (
                "pentesting",
                "Isolated network with tools access",
                SecurityProfile {
                    name: "pentesting".to_string(),
                    sandbox_enabled: true,
                    network_isolation: NetworkIsolation {
                        mode: IsolationMode::Internal,
                        allow_host_access: true,
                        allow_internet: true,
                        isolated_network_id: Some("pentest-net".to_string()),
                        mac_address: None,
                    },
                    tor_enabled: false,
                    vpn_config: None,
                    proxy_config: Some(ProxyConfig {
                        proxy_type: ProxyType::Socks5,
                        host: "127.0.0.1".to_string(),
                        port: 1080,
                        username: None,
                        password: None,
                        chain: Vec::new(),
                    }),
                    firewall_rules: Self::default_firewall_rules(),
                    virtual_devices: Self::default_virtual_devices(),
                },
            ),
        ]
    }

    fn default_firewall_rules() -> Vec<FirewallRule> {
        vec![
            FirewallRule {
                action: FirewallAction::Allow,
                direction: TrafficDirection::Outbound,
                protocol: Some("tcp".to_string()),
                source: None,
                destination: None,
                port: Some(443),
                port_range: None,
                description: "Allow HTTPS".to_string(),
            },
            FirewallRule {
                action: FirewallAction::Allow,
                direction: TrafficDirection::Outbound,
                protocol: Some("tcp".to_string()),
                source: None,
                destination: None,
                port: Some(80),
                port_range: None,
                description: "Allow HTTP".to_string(),
            },
            FirewallRule {
                action: FirewallAction::Allow,
                direction: TrafficDirection::Outbound,
                protocol: Some("udp".to_string()),
                source: None,
                destination: None,
                port: Some(53),
                port_range: None,
                description: "Allow DNS".to_string(),
            },
        ]
    }

    fn default_virtual_devices() -> Vec<VirtualDevice> {
        vec![
            VirtualDevice {
                device_type: VirtualDeviceType::NetworkAdapter,
                name: "virtio-net".to_string(),
                enabled: true,
                passthrough: false,
                isolated: false,
            },
            VirtualDevice {
                device_type: VirtualDeviceType::UsbController,
                name: "usb-tablet".to_string(),
                enabled: true,
                passthrough: false,
                isolated: true,
            },
        ]
    }

    /// Generate QEMU arguments for security profile
    pub fn generate_qemu_security_args(&self, profile: &SecurityProfile) -> Vec<String> {
        let mut args = Vec::new();

        // Sandbox mode
        if profile.sandbox_enabled {
            args.extend(["-sandbox".to_string(), "on".to_string()]);
        }

        // Network isolation
        match &profile.network_isolation.mode {
            IsolationMode::Full => {
                args.extend(["-nic".to_string(), "none".to_string()]);
            }
            IsolationMode::HostOnly => {
                args.extend([
                    "-netdev".to_string(),
                    "user,id=hostonly,restrict=on".to_string(),
                    "-device".to_string(),
                    "virtio-net-pci,netdev=hostonly".to_string(),
                ]);
            }
            IsolationMode::Internal => {
                let net_id = profile.network_isolation.isolated_network_id.as_deref().unwrap_or("internal");
                args.extend([
                    "-netdev".to_string(),
                    format!("socket,id={},mcast=230.0.0.1:1234", net_id),
                    "-device".to_string(),
                    format!("virtio-net-pci,netdev={}", net_id),
                ]);
            }
            IsolationMode::TorOnly => {
                // Route through Tor SOCKS proxy
                args.extend([
                    "-netdev".to_string(),
                    format!(
                        "user,id=tornet,hostfwd=tcp::2222-:22,guestfwd=tcp:10.0.2.100:9050-cmd:nc 127.0.0.1 {}",
                        self.tor_config.socks_port
                    ),
                    "-device".to_string(),
                    "virtio-net-pci,netdev=tornet".to_string(),
                ]);
            }
            IsolationMode::VpnOnly => {
                // VPN configuration would be handled by the guest OS
                args.extend([
                    "-netdev".to_string(),
                    "user,id=vpnnet,restrict=off".to_string(),
                    "-device".to_string(),
                    "virtio-net-pci,netdev=vpnnet".to_string(),
                ]);
            }
            _ => {}
        }

        // Custom MAC address
        if let Some(mac) = &profile.network_isolation.mac_address {
            // Find the device arg and append mac
            for i in 0..args.len() {
                if args[i].starts_with("virtio-net-pci") {
                    args[i] = format!("{},mac={}", args[i], mac);
                    break;
                }
            }
        }

        args
    }

    /// Generate torrc configuration
    pub fn generate_torrc(&self, vm_name: &str) -> String {
        let config = &self.tor_config;
        let mut torrc = format!(
            r#"# n01d Machine Tor Configuration for {}
SocksPort {}
ControlPort {}
DNSPort {}
AutomapHostsOnResolve 1
AutomapHostsSuffixes .onion,.exit
VirtualAddrNetworkIPv4 10.192.0.0/10
"#,
            vm_name, config.socks_port, config.control_port, config.dns_port
        );

        if config.transparent_proxy {
            torrc.push_str("TransPort 9040\n");
        }

        if config.bridge_enabled && !config.bridges.is_empty() {
            torrc.push_str("UseBridges 1\n");
            for bridge in &config.bridges {
                torrc.push_str(&format!("Bridge {}\n", bridge));
            }
        }

        if let Some(exit_nodes) = &config.exit_nodes {
            torrc.push_str(&format!("ExitNodes {}\n", exit_nodes.join(",")));
        }

        if let Some(exclude) = &config.exclude_exit_nodes {
            torrc.push_str(&format!("ExcludeExitNodes {}\n", exclude.join(",")));
        }

        if config.strict_nodes {
            torrc.push_str("StrictNodes 1\n");
        }

        torrc.push_str(&format!("NewCircuitPeriod {}\n", config.new_circuit_period));

        torrc
    }

    /// Generate WireGuard configuration
    pub fn generate_wireguard_config(vpn: &VpnConfig) -> String {
        format!(
            r#"[Interface]
PrivateKey = <YOUR_PRIVATE_KEY>
Address = 10.0.0.2/24
DNS = 1.1.1.1

[Peer]
PublicKey = <SERVER_PUBLIC_KEY>
Endpoint = {}:{}
AllowedIPs = 0.0.0.0/0
PersistentKeepalive = 25
"#,
            vpn.server.as_deref().unwrap_or("vpn.example.com"),
            vpn.port
        )
    }

    /// Generate iptables rules for VM network isolation
    pub fn generate_iptables_rules(&self, profile: &SecurityProfile, vm_interface: &str) -> Vec<String> {
        let mut rules = Vec::new();
        
        // Flush existing rules for this VM
        rules.push(format!("iptables -F n01d-{}", profile.name));
        rules.push(format!("iptables -N n01d-{} 2>/dev/null || true", profile.name));

        for rule in &profile.firewall_rules {
            let action = match rule.action {
                FirewallAction::Allow => "ACCEPT",
                FirewallAction::Deny => "REJECT",
                FirewallAction::Drop => "DROP",
                FirewallAction::Log => "LOG",
            };

            let direction = match rule.direction {
                TrafficDirection::Inbound => "-i",
                TrafficDirection::Outbound => "-o",
                TrafficDirection::Both => "-i", // Will add both
            };

            let mut cmd = format!("iptables -A n01d-{} {} {}", profile.name, direction, vm_interface);

            if let Some(proto) = &rule.protocol {
                cmd.push_str(&format!(" -p {}", proto));
            }

            if let Some(src) = &rule.source {
                cmd.push_str(&format!(" -s {}", src));
            }

            if let Some(dst) = &rule.destination {
                cmd.push_str(&format!(" -d {}", dst));
            }

            if let Some(port) = rule.port {
                cmd.push_str(&format!(" --dport {}", port));
            }

            if let Some((start, end)) = rule.port_range {
                cmd.push_str(&format!(" --dport {}:{}", start, end));
            }

            cmd.push_str(&format!(" -j {} -m comment --comment \"{}\"", action, rule.description));
            rules.push(cmd);

            // Add outbound rule too if direction is Both
            if matches!(rule.direction, TrafficDirection::Both) {
                let mut cmd_out = format!("iptables -A n01d-{} -o {}", profile.name, vm_interface);
                if let Some(proto) = &rule.protocol {
                    cmd_out.push_str(&format!(" -p {}", proto));
                }
                cmd_out.push_str(&format!(" -j {}", action));
                rules.push(cmd_out);
            }
        }

        rules
    }
}

/// Helper to check if Tor is running
pub fn check_tor_status() -> bool {
    Command::new("pgrep")
        .arg("tor")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Start Tor with custom config
pub fn start_tor(config_path: &str) -> Result<(), String> {
    Command::new("tor")
        .args(["-f", config_path])
        .spawn()
        .map_err(|e| format!("Failed to start Tor: {}", e))?;
    Ok(())
}

/// Get new Tor circuit
pub fn new_tor_circuit(control_port: u16) -> Result<(), String> {
    use std::io::Write;
    use std::net::TcpStream;

    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", control_port))
        .map_err(|e| format!("Failed to connect to Tor control: {}", e))?;

    stream
        .write_all(b"AUTHENTICATE\r\nSIGNAL NEWNYM\r\nQUIT\r\n")
        .map_err(|e| format!("Failed to send signal: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_profiles() {
        let presets = SecurityManager::get_preset_profiles();
        assert!(!presets.is_empty());
        assert!(presets.iter().any(|(name, _, _)| *name == "paranoid"));
    }

    #[test]
    fn test_qemu_args_generation() {
        let manager = SecurityManager::new(PathBuf::from("/tmp"));
        let profile = SecurityProfile {
            name: "test".to_string(),
            sandbox_enabled: true,
            network_isolation: NetworkIsolation {
                mode: IsolationMode::Full,
                ..Default::default()
            },
            ..Default::default()
        };

        let args = manager.generate_qemu_security_args(&profile);
        assert!(args.contains(&"-sandbox".to_string()));
        assert!(args.contains(&"on".to_string()));
        assert!(args.contains(&"-nic".to_string()));
        assert!(args.contains(&"none".to_string()));
    }
}

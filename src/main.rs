//! n01d Machine - Secure VM Console

mod vm;
mod sandbox;
mod network;
mod gui;

use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;

const BANNER: &str = r#"
                ___  __    __                      __    _          
   ____  ____  <  / / /___/ /  ____ ___  ____ ____/ /_  (_)___  ___ 
  / __ \/ __ \ / / / __  / /  / __ `__ \/ __ `/ __  / / / / __ \/ _ \
 / / / / /_/ // / / /_/ / /  / / / / / / /_/ / /_/ / /_/ / / / /  __/
/_/ /_/\____//_/  \__,_/_/  /_/ /_/ /_/\__,_/\__,_/\____/_/ /_/\___/ 
                                                                      
    n01d Machine v1.0.0 | Secure Virtualization Platform
    ═══════════════════════════════════════════════════════
"#;

#[derive(Parser)]
#[command(name = "n01d")]
#[command(about = "n01d Machine - Secure virtualization with sandboxing")]
#[command(version = "1.0.0")]
struct Cli {
    /// Launch GUI mode
    #[arg(long)]
    gui: bool,
    
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List all VMs
    List {
        /// Show detailed info
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Create a new VM
    Create {
        /// VM name
        #[arg(short, long)]
        name: String,
        
        /// RAM allocation (e.g., 2G)
        #[arg(long, default_value = "2G")]
        ram: String,
        
        /// Disk size (e.g., 20G)
        #[arg(long, default_value = "20G")]
        disk: String,
        
        /// Number of CPUs
        #[arg(long, default_value = "2")]
        cpus: u32,
        
        /// ISO file for installation
        #[arg(long)]
        iso: Option<PathBuf>,
        
        /// VM template to use
        #[arg(long)]
        template: Option<String>,
    },
    
    /// Start a VM
    Start {
        /// VM name
        name: String,
        
        /// Run in isolated mode
        #[arg(long)]
        isolated: bool,
        
        /// Network mode (nat, isolated, none, bridge)
        #[arg(long, default_value = "nat")]
        network: String,
        
        /// Headless mode
        #[arg(long)]
        headless: bool,
    },
    
    /// Stop a VM
    Stop {
        /// VM name
        name: String,
        
        /// Force stop
        #[arg(short, long)]
        force: bool,
    },
    
    /// Create a sandbox environment
    Sandbox {
        /// Sandbox name
        #[arg(short, long)]
        name: String,
        
        /// Isolation level (minimal, low, medium, high, max)
        #[arg(long, default_value = "medium")]
        isolation: String,
        
        /// Base image
        #[arg(long)]
        image: Option<String>,
        
        /// Command to run
        #[arg(long)]
        cmd: Option<String>,
    },
    
    /// Take a snapshot
    Snapshot {
        /// VM name
        vm: String,
        
        /// Snapshot name
        #[arg(short, long)]
        name: String,
    },
    
    /// Restore from snapshot
    Restore {
        /// VM name
        vm: String,
        
        /// Snapshot name
        #[arg(short, long)]
        snapshot: String,
    },
    
    /// Network management
    Network {
        #[command(subcommand)]
        command: NetworkCommands,
    },
    
    /// Configuration
    Config {
        /// Show current config
        #[arg(long)]
        show: bool,
        
        /// Set a config value
        #[arg(long)]
        set: Option<String>,
    },
    
    /// Show system dashboard
    Dashboard,
    
    /// VPN management
    Vpn {
        #[command(subcommand)]
        command: VpnCommands,
    },
}

#[derive(Subcommand)]
enum NetworkCommands {
    /// List virtual networks
    List,
    
    /// Create a virtual network
    Create {
        /// Network name
        #[arg(short, long)]
        name: String,
        
        /// Network mode
        #[arg(long, default_value = "nat")]
        mode: String,
        
        /// Subnet (e.g., 10.0.0.0/24)
        #[arg(long)]
        subnet: Option<String>,
    },
    
    /// Delete a virtual network
    Delete {
        /// Network name
        name: String,
    },
    
    /// Inspect network traffic
    Inspect {
        /// Network or VM name
        target: String,
        
        /// Output file for pcap
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum VpnCommands {
    /// Connect to VPN
    Connect {
        /// VPN config file
        #[arg(short, long)]
        config: PathBuf,
        
        /// VPN type (openvpn, wireguard)
        #[arg(long, default_value = "openvpn")]
        vpn_type: String,
        
        /// Interface name (for WireGuard)
        #[arg(long)]
        interface: Option<String>,
    },
    
    /// Disconnect VPN
    Disconnect {
        /// VPN type
        #[arg(long, default_value = "openvpn")]
        vpn_type: String,
        
        /// Interface name (for WireGuard)
        #[arg(long)]
        interface: Option<String>,
    },
    
    /// Start Tor proxy
    Tor,
}

fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    if cli.gui {
        println!("{}", BANNER.cyan());
        println!("{}", "Launching n01d GUI...".green());
        gui::launch_gui()?;
        return Ok(());
    }
    
    match cli.command {
        Some(Commands::List { verbose }) => {
            println!("{}", BANNER.cyan());
            vm::list_vms(verbose)?;
        }
        
        Some(Commands::Create { name, ram, disk, cpus, iso, template }) => {
            println!("{}", BANNER.cyan());
            println!("{} Creating VM '{}'...", "[n01d]".blue(), name);
            
            let config = vm::VmConfig {
                name: name.clone(),
                ram,
                disk,
                cpus,
                iso,
                template,
            };
            
            vm::create_vm(config)?;
            println!("{} VM '{}' created successfully!", "[+]".green(), name);
        }
        
        Some(Commands::Start { name, isolated, network, headless }) => {
            println!("{}", BANNER.cyan());
            println!("{} Starting VM '{}'...", "[n01d]".blue(), name);
            
            if isolated {
                println!("{} Running in isolated mode", "[!]".yellow());
            }
            
            vm::start_vm(&name, isolated, &network, headless)?;
        }
        
        Some(Commands::Stop { name, force }) => {
            println!("{} Stopping VM '{}'...", "[n01d]".blue(), name);
            vm::stop_vm(&name, force)?;
            println!("{} VM '{}' stopped", "[+]".green(), name);
        }
        
        Some(Commands::Sandbox { name, isolation, image, cmd }) => {
            println!("{}", BANNER.cyan());
            println!("{} Creating sandbox '{}'...", "[n01d]".blue(), name);
            println!("{} Isolation level: {}", "[*]".blue(), isolation.yellow());
            
            sandbox::create_sandbox(&name, &isolation, image.as_deref(), cmd.as_deref())?;
        }
        
        Some(Commands::Snapshot { vm, name }) => {
            println!("{} Creating snapshot '{}' for VM '{}'...", "[n01d]".blue(), name, vm);
            vm::create_snapshot(&vm, &name)?;
            println!("{} Snapshot created successfully!", "[+]".green());
        }
        
        Some(Commands::Restore { vm, snapshot }) => {
            println!("{} Restoring VM '{}' to snapshot '{}'...", "[n01d]".blue(), vm, snapshot);
            vm::restore_snapshot(&vm, &snapshot)?;
            println!("{} VM restored successfully!", "[+]".green());
        }
        
        Some(Commands::Network { command }) => {
            match command {
                NetworkCommands::List => {
                    network::list_networks()?;
                }
                NetworkCommands::Create { name, mode, subnet } => {
                    println!("{} Creating network '{}'...", "[n01d]".blue(), name);
                    network::create_network(&name, &mode, subnet.as_deref())?;
                }
                NetworkCommands::Delete { name } => {
                    println!("{} Deleting network '{}'...", "[n01d]".blue(), name);
                    network::delete_network(&name)?;
                }
                NetworkCommands::Inspect { target, output } => {
                    println!("{} Inspecting traffic for '{}'...", "[n01d]".blue(), target);
                    network::inspect_traffic(&target, output.as_deref())?;
                }
            }
        }
        
        Some(Commands::Config { show, set }) => {
            if show {
                vm::show_config()?;
            } else if let Some(value) = set {
                vm::set_config(&value)?;
            }
        }
        
        Some(Commands::Dashboard) => {
            println!("{}", BANNER.cyan());
            gui::dashboard::print_dashboard()?;
        }
        
        Some(Commands::Vpn { command }) => {
            match command {
                VpnCommands::Connect { config, vpn_type, interface } => {
                    match vpn_type.to_lowercase().as_str() {
                        "wireguard" | "wg" => {
                            let iface = interface.unwrap_or_else(|| "wg0".to_string());
                            network::vpn::connect_wireguard(&iface, &config.to_string_lossy())?;
                        }
                        _ => {
                            network::vpn::connect_openvpn(&config.to_string_lossy())?;
                        }
                    }
                }
                VpnCommands::Disconnect { vpn_type, interface } => {
                    let provider = match vpn_type.to_lowercase().as_str() {
                        "wireguard" | "wg" => network::vpn::VpnProvider::WireGuard,
                        _ => network::vpn::VpnProvider::OpenVPN,
                    };
                    network::vpn::disconnect_vpn(provider, interface.as_deref())?;
                }
                VpnCommands::Tor => {
                    network::proxy::start_tor_proxy()?;
                }
            }
        }
        
        None => {
            println!("{}", BANNER.cyan());
            println!("Use --help for usage information");
            println!("Use --gui to launch the graphical interface");
        }
    }
    
    Ok(())
}

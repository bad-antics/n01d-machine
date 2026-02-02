//! Sandbox Module - Secure isolation for testing

use anyhow::{Result, Context};
use std::process::Command;
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IsolationLevel {
    Minimal,
    Low,
    Medium,
    High,
    Maximum,
}

impl std::str::FromStr for IsolationLevel {
    type Err = anyhow::Error;
    
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "minimal" | "min" => Ok(IsolationLevel::Minimal),
            "low" => Ok(IsolationLevel::Low),
            "medium" | "med" => Ok(IsolationLevel::Medium),
            "high" => Ok(IsolationLevel::High),
            "maximum" | "max" => Ok(IsolationLevel::Maximum),
            _ => anyhow::bail!("Unknown isolation level: {}", s),
        }
    }
}

#[derive(Debug)]
pub struct SandboxConfig {
    pub name: String,
    pub level: IsolationLevel,
    pub image: Option<String>,
    pub command: Option<String>,
    pub network: NetworkConfig,
    pub filesystem: FilesystemConfig,
    pub seccomp: SeccompConfig,
    pub capabilities: CapConfig,
}

#[derive(Debug)]
pub struct NetworkConfig {
    pub enabled: bool,
    pub mode: String, // none, isolated, nat
}

#[derive(Debug)]
pub struct FilesystemConfig {
    pub mode: String,    // full, readonly, tmpfs, memory
    pub mounts: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct SeccompConfig {
    pub enabled: bool,
    pub profile: String, // permissive, standard, strict
}

#[derive(Debug)]
pub struct CapConfig {
    pub drop_all: bool,
    pub allowed: Vec<String>,
}

impl SandboxConfig {
    pub fn from_level(name: &str, level: IsolationLevel) -> Self {
        match level {
            IsolationLevel::Minimal => Self {
                name: name.to_string(),
                level,
                image: None,
                command: None,
                network: NetworkConfig { enabled: true, mode: "nat".into() },
                filesystem: FilesystemConfig { mode: "full".into(), mounts: vec![] },
                seccomp: SeccompConfig { enabled: false, profile: "permissive".into() },
                capabilities: CapConfig { drop_all: false, allowed: vec![] },
            },
            IsolationLevel::Low => Self {
                name: name.to_string(),
                level,
                image: None,
                command: None,
                network: NetworkConfig { enabled: true, mode: "nat".into() },
                filesystem: FilesystemConfig { mode: "shared".into(), mounts: vec![] },
                seccomp: SeccompConfig { enabled: true, profile: "permissive".into() },
                capabilities: CapConfig { drop_all: false, allowed: vec!["CAP_NET_BIND_SERVICE".into()] },
            },
            IsolationLevel::Medium => Self {
                name: name.to_string(),
                level,
                image: None,
                command: None,
                network: NetworkConfig { enabled: true, mode: "isolated".into() },
                filesystem: FilesystemConfig { mode: "readonly".into(), mounts: vec![] },
                seccomp: SeccompConfig { enabled: true, profile: "standard".into() },
                capabilities: CapConfig { drop_all: true, allowed: vec![] },
            },
            IsolationLevel::High => Self {
                name: name.to_string(),
                level,
                image: None,
                command: None,
                network: NetworkConfig { enabled: false, mode: "none".into() },
                filesystem: FilesystemConfig { mode: "tmpfs".into(), mounts: vec![] },
                seccomp: SeccompConfig { enabled: true, profile: "strict".into() },
                capabilities: CapConfig { drop_all: true, allowed: vec![] },
            },
            IsolationLevel::Maximum => Self {
                name: name.to_string(),
                level,
                image: None,
                command: None,
                network: NetworkConfig { enabled: false, mode: "none".into() },
                filesystem: FilesystemConfig { mode: "memory".into(), mounts: vec![] },
                seccomp: SeccompConfig { enabled: true, profile: "strict".into() },
                capabilities: CapConfig { drop_all: true, allowed: vec![] },
            },
        }
    }
}

pub fn create_sandbox(
    name: &str,
    isolation: &str,
    image: Option<&str>,
    cmd: Option<&str>,
) -> Result<()> {
    use colored::*;
    
    let level: IsolationLevel = isolation.parse()?;
    let mut config = SandboxConfig::from_level(name, level);
    config.image = image.map(String::from);
    config.command = cmd.map(String::from);
    
    println!("{} Creating sandbox with {} isolation", "[*]".blue(), isolation.yellow());
    
    // Create sandbox directory
    let sandbox_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("NullSec-Sandboxes")
        .join(name);
    
    fs::create_dir_all(&sandbox_dir)?;
    
    // Generate unshare command based on isolation level
    let mut unshare_cmd = build_unshare_command(&config);
    
    // If we have an image, use it
    if let Some(img) = &config.image {
        println!("{} Using base image: {}", "[*]".blue(), img);
    }
    
    // If we have a command, add it
    if let Some(c) = &config.command {
        unshare_cmd.push_str(&format!(" -- {}", c));
    } else {
        unshare_cmd.push_str(" -- /bin/bash");
    }
    
    // Save sandbox config
    let config_path = sandbox_dir.join("sandbox.toml");
    let config_toml = format!(r#"
[sandbox]
name = "{}"
isolation = "{}"
created = "{}"

[network]
enabled = {}
mode = "{}"

[filesystem]
mode = "{}"

[seccomp]
enabled = {}
profile = "{}"

[capabilities]
drop_all = {}
"#, 
        config.name,
        isolation,
        chrono::Utc::now().to_rfc3339(),
        config.network.enabled,
        config.network.mode,
        config.filesystem.mode,
        config.seccomp.enabled,
        config.seccomp.profile,
        config.capabilities.drop_all,
    );
    
    fs::write(&config_path, config_toml)?;
    
    // Create launcher script
    let launcher_path = sandbox_dir.join("enter.sh");
    let launcher_script = format!(r#"#!/bin/bash
# NullSec Sandbox Launcher - {}
# Isolation Level: {}

echo "[*] Entering sandbox '{}' with {} isolation"
echo "[!] Press Ctrl+D or type 'exit' to leave sandbox"
echo ""

{}
"#, name, isolation, name, isolation, unshare_cmd);
    
    fs::write(&launcher_path, launcher_script)?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&launcher_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&launcher_path, perms)?;
    }
    
    println!("{} Sandbox '{}' created at: {}", "[+]".green(), name, sandbox_dir.display());
    println!("{} To enter sandbox: {}/enter.sh", "[*]".blue(), sandbox_dir.display());
    
    // Optionally start the sandbox immediately
    println!("\n{} Enter sandbox now? [y/N]", "[?]".cyan());
    
    Ok(())
}

fn build_unshare_command(config: &SandboxConfig) -> String {
    let mut cmd = String::from("unshare");
    
    // Always create new PID namespace
    cmd.push_str(" --pid --fork");
    
    // Mount namespace for filesystem isolation
    if config.filesystem.mode != "full" {
        cmd.push_str(" --mount");
    }
    
    // User namespace for capability dropping
    if config.capabilities.drop_all {
        cmd.push_str(" --user --map-root-user");
    }
    
    // Network namespace
    if !config.network.enabled || config.network.mode == "none" {
        cmd.push_str(" --net");
    }
    
    // UTS namespace (hostname)
    cmd.push_str(" --uts");
    
    // IPC namespace
    cmd.push_str(" --ipc");
    
    // Cgroup namespace
    cmd.push_str(" --cgroup");
    
    cmd
}

/// Seccomp filter profiles
pub mod seccomp_profiles {
    pub const PERMISSIVE: &[&str] = &[];
    
    pub const STANDARD_BLOCKED: &[&str] = &[
        "kexec_load",
        "kexec_file_load",
        "init_module",
        "finit_module",
        "delete_module",
        "reboot",
        "swapon",
        "swapoff",
        "mount",
        "umount2",
        "pivot_root",
        "ptrace",
    ];
    
    pub const STRICT_ALLOWED: &[&str] = &[
        "read", "write", "open", "close", "stat", "fstat", "lstat",
        "poll", "lseek", "mmap", "mprotect", "munmap", "brk",
        "rt_sigaction", "rt_sigprocmask", "ioctl", "access",
        "pipe", "select", "sched_yield", "mremap", "msync",
        "mincore", "madvise", "shmget", "shmat", "shmctl",
        "dup", "dup2", "pause", "nanosleep", "getitimer",
        "alarm", "setitimer", "getpid", "sendfile", "socket",
        "connect", "accept", "sendto", "recvfrom", "sendmsg",
        "recvmsg", "shutdown", "bind", "listen", "getsockname",
        "getpeername", "socketpair", "setsockopt", "getsockopt",
        "clone", "fork", "vfork", "execve", "exit", "wait4",
        "kill", "uname", "fcntl", "flock", "fsync", "fdatasync",
        "truncate", "ftruncate", "getdents", "getcwd", "chdir",
        "fchdir", "rename", "mkdir", "rmdir", "creat", "link",
        "unlink", "symlink", "readlink", "chmod", "fchmod",
        "chown", "fchown", "lchown", "umask", "gettimeofday",
        "getrlimit", "getrusage", "sysinfo", "times", "getuid",
        "syslog", "getgid", "setuid", "setgid", "geteuid",
        "getegid", "setpgid", "getppid", "getpgrp", "setsid",
        "setreuid", "setregid", "getgroups", "setgroups",
        "setresuid", "getresuid", "setresgid", "getresgid",
        "getpgid", "setfsuid", "setfsgid", "getsid", "capget",
        "rt_sigpending", "rt_sigtimedwait", "rt_sigqueueinfo",
        "rt_sigsuspend", "sigaltstack", "utime", "mknod",
        "personality", "ustat", "statfs", "fstatfs", "sysfs",
        "getpriority", "setpriority", "sched_setparam",
        "sched_getparam", "sched_setscheduler", "sched_getscheduler",
        "sched_get_priority_max", "sched_get_priority_min",
        "sched_rr_get_interval", "mlock", "munlock", "mlockall",
        "munlockall", "vhangup", "prctl", "arch_prctl",
        "setrlimit", "sync", "acct", "settimeofday",
        "sethostname", "setdomainname", "ioperm", "iopl",
        "create_module", "get_kernel_syms", "query_module",
        "quotactl", "nfsservctl", "getpmsg", "putpmsg",
        "afs_syscall", "tuxcall", "security", "gettid",
        "readahead", "setxattr", "lsetxattr", "fsetxattr",
        "getxattr", "lgetxattr", "fgetxattr", "listxattr",
        "llistxattr", "flistxattr", "removexattr", "lremovexattr",
        "fremovexattr", "tkill", "time", "futex", "sched_setaffinity",
        "sched_getaffinity", "set_thread_area", "io_setup",
        "io_destroy", "io_getevents", "io_submit", "io_cancel",
        "get_thread_area", "lookup_dcookie", "epoll_create",
        "epoll_ctl_old", "epoll_wait_old", "remap_file_pages",
        "getdents64", "set_tid_address", "restart_syscall",
        "semtimedop", "fadvise64", "timer_create", "timer_settime",
        "timer_gettime", "timer_getoverrun", "timer_delete",
        "clock_settime", "clock_gettime", "clock_getres",
        "clock_nanosleep", "exit_group", "epoll_wait", "epoll_ctl",
        "tgkill", "utimes", "vserver", "mbind", "set_mempolicy",
        "get_mempolicy", "mq_open", "mq_unlink", "mq_timedsend",
        "mq_timedreceive", "mq_notify", "mq_getsetattr",
        "kexec_load", "waitid", "add_key", "request_key", "keyctl",
        "ioprio_set", "ioprio_get", "inotify_init", "inotify_add_watch",
        "inotify_rm_watch", "migrate_pages", "openat", "mkdirat",
        "mknodat", "fchownat", "futimesat", "newfstatat", "unlinkat",
        "renameat", "linkat", "symlinkat", "readlinkat", "fchmodat",
        "faccessat", "pselect6", "ppoll", "unshare", "set_robust_list",
        "get_robust_list", "splice", "tee", "sync_file_range", "vmsplice",
        "move_pages", "utimensat", "epoll_pwait", "signalfd", "timerfd_create",
        "eventfd", "fallocate", "timerfd_settime", "timerfd_gettime",
        "accept4", "signalfd4", "eventfd2", "epoll_create1", "dup3",
        "pipe2", "inotify_init1", "preadv", "pwritev", "rt_tgsigqueueinfo",
        "perf_event_open", "recvmmsg", "fanotify_init", "fanotify_mark",
        "prlimit64", "name_to_handle_at", "open_by_handle_at", "clock_adjtime",
        "syncfs", "sendmmsg", "setns", "getcpu", "process_vm_readv",
        "process_vm_writev", "kcmp", "finit_module", "sched_setattr",
        "sched_getattr", "renameat2", "seccomp", "getrandom", "memfd_create",
        "execveat", "userfaultfd", "membarrier", "mlock2", "copy_file_range",
        "preadv2", "pwritev2",
    ];
}

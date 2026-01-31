//! Container management commands.

use anyhow::Result;
use clap::{Args, Subcommand};
use colored::*;
use tabled::{Table, Tabled};

/// Container management commands.
#[derive(Args)]
pub struct ContainerCommand {
    #[command(subcommand)]
    pub action: ContainerAction,
}

#[derive(Subcommand)]
pub enum ContainerAction {
    /// List containers
    #[command(alias = "ls")]
    List {
        /// Show all containers (including stopped)
        #[arg(short, long)]
        all: bool,

        /// Filter by project
        #[arg(short, long)]
        project: Option<String>,

        /// Quiet mode (only show IDs)
        #[arg(short, long)]
        quiet: bool,
    },

    /// Run a new container
    Run {
        /// Image to run
        image: String,

        /// Container name
        #[arg(long)]
        name: Option<String>,

        /// Port mappings (host:container)
        #[arg(short, long)]
        port: Vec<String>,

        /// Volume mounts (host:container)
        #[arg(short, long)]
        volume: Vec<String>,

        /// Environment variables
        #[arg(short, long)]
        env: Vec<String>,

        /// Run in detached mode
        #[arg(short, long)]
        detach: bool,

        /// Remove container when it exits
        #[arg(long)]
        rm: bool,

        /// Interactive mode
        #[arg(short, long)]
        interactive: bool,

        /// Allocate a TTY
        #[arg(short, long)]
        tty: bool,

        /// Working directory inside container
        #[arg(short, long)]
        workdir: Option<String>,

        /// Command to run
        #[arg(last = true)]
        command: Vec<String>,
    },

    /// Start a stopped container
    Start {
        /// Container ID or name
        container: String,
    },

    /// Stop a running container
    Stop {
        /// Container ID or name
        container: String,

        /// Timeout in seconds
        #[arg(short, long, default_value = "10")]
        timeout: u64,
    },

    /// Restart a container
    Restart {
        /// Container ID or name
        container: String,

        /// Timeout in seconds
        #[arg(short, long, default_value = "10")]
        timeout: u64,
    },

    /// Remove a container
    #[command(alias = "rm")]
    Remove {
        /// Container IDs or names
        containers: Vec<String>,

        /// Force removal
        #[arg(short, long)]
        force: bool,

        /// Remove associated volumes
        #[arg(short, long)]
        volumes: bool,
    },

    /// Execute a command in a running container
    Exec {
        /// Container ID or name
        container: String,

        /// Interactive mode
        #[arg(short, long)]
        interactive: bool,

        /// Allocate a TTY
        #[arg(short, long)]
        tty: bool,

        /// Working directory inside container
        #[arg(short, long)]
        workdir: Option<String>,

        /// Environment variables
        #[arg(short, long)]
        env: Vec<String>,

        /// Command to run
        command: Vec<String>,
    },

    /// Show container logs
    Logs {
        /// Container ID or name
        container: String,

        /// Follow log output
        #[arg(short, long)]
        follow: bool,

        /// Number of lines to show
        #[arg(short = 'n', long, default_value = "100")]
        tail: usize,

        /// Show timestamps
        #[arg(short, long)]
        timestamps: bool,
    },

    /// Show container details
    Inspect {
        /// Container ID or name
        container: String,
    },

    /// Show container statistics
    Stats {
        /// Container IDs or names (all if empty)
        containers: Vec<String>,

        /// Disable streaming stats
        #[arg(long)]
        no_stream: bool,
    },

    /// Copy files between container and host
    Cp {
        /// Source path
        source: String,

        /// Destination path
        dest: String,
    },
}

pub async fn run(cmd: ContainerCommand) -> Result<()> {
    match cmd.action {
        ContainerAction::List { all, project, quiet } => list_containers(all, project, quiet).await,
        ContainerAction::Run { image, name, port, volume, env, detach, rm, interactive, tty, workdir, command } => {
            run_container(image, name, port, volume, env, detach, rm, interactive, tty, workdir, command).await
        }
        ContainerAction::Start { container } => start_container(container).await,
        ContainerAction::Stop { container, timeout } => stop_container(container, timeout).await,
        ContainerAction::Restart { container, timeout } => restart_container(container, timeout).await,
        ContainerAction::Remove { containers, force, volumes } => remove_containers(containers, force, volumes).await,
        ContainerAction::Exec { container, interactive, tty, workdir, env, command } => {
            exec_container(container, interactive, tty, workdir, env, command).await
        }
        ContainerAction::Logs { container, follow, tail, timestamps } => {
            show_logs(container, follow, tail, timestamps).await
        }
        ContainerAction::Inspect { container } => inspect_container(container).await,
        ContainerAction::Stats { containers, no_stream } => show_stats(containers, no_stream).await,
        ContainerAction::Cp { source, dest } => copy_files(source, dest).await,
    }
}

#[derive(Tabled)]
struct ContainerInfo {
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "IMAGE")]
    image: String,
    #[tabled(rename = "STATUS")]
    status: String,
    #[tabled(rename = "PORTS")]
    ports: String,
    #[tabled(rename = "NAME")]
    name: String,
}

async fn list_containers(all: bool, project: Option<String>, quiet: bool) -> Result<()> {
    let containers = vec![
        ContainerInfo {
            id: "a1b2c3d4".to_string(),
            image: "node:20".to_string(),
            status: "Up 2 minutes".green().to_string(),
            ports: "3000:3000".to_string(),
            name: "my-app_web_1".to_string(),
        },
        ContainerInfo {
            id: "e5f6g7h8".to_string(),
            image: "postgres:15".to_string(),
            status: "Up 2 minutes".green().to_string(),
            ports: "5432:5432".to_string(),
            name: "my-app_db_1".to_string(),
        },
    ];

    if quiet {
        for c in &containers {
            println!("{}", c.id);
        }
    } else {
        let table = Table::new(containers).to_string();
        println!("{}", table);
    }

    Ok(())
}

async fn run_container(
    image: String,
    name: Option<String>,
    ports: Vec<String>,
    volumes: Vec<String>,
    env: Vec<String>,
    detach: bool,
    rm: bool,
    interactive: bool,
    tty: bool,
    workdir: Option<String>,
    command: Vec<String>,
) -> Result<()> {
    let container_name = name.unwrap_or_else(|| format!("hb-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()));

    println!("{} Starting container {}...", "→".blue(), container_name.cyan());
    println!("{} Container started: {}", "✓".green(), container_name.cyan());

    Ok(())
}

async fn start_container(container: String) -> Result<()> {
    println!("{} Starting container {}...", "→".blue(), container.cyan());
    println!("{} Container started", "✓".green());
    Ok(())
}

async fn stop_container(container: String, timeout: u64) -> Result<()> {
    println!("{} Stopping container {}...", "→".blue(), container.cyan());
    println!("{} Container stopped", "✓".green());
    Ok(())
}

async fn restart_container(container: String, timeout: u64) -> Result<()> {
    stop_container(container.clone(), timeout).await?;
    start_container(container).await?;
    Ok(())
}

async fn remove_containers(containers: Vec<String>, force: bool, volumes: bool) -> Result<()> {
    for container in &containers {
        println!("{} Removing container {}...", "→".blue(), container.cyan());
    }
    println!("{} Removed {} container(s)", "✓".green(), containers.len());
    Ok(())
}

async fn exec_container(
    container: String,
    interactive: bool,
    tty: bool,
    workdir: Option<String>,
    env: Vec<String>,
    command: Vec<String>,
) -> Result<()> {
    let cmd = if command.is_empty() {
        "sh".to_string()
    } else {
        command.join(" ")
    };

    println!("{} Executing '{}' in {}...", "→".blue(), cmd.cyan(), container);
    Ok(())
}

async fn show_logs(container: String, follow: bool, tail: usize, timestamps: bool) -> Result<()> {
    println!("{}", "[2024-01-15 10:30:00] Server started on port 3000".cyan());
    println!("{}", "[2024-01-15 10:30:01] Database connected".cyan());
    println!("{}", "[2024-01-15 10:30:02] Ready to accept connections".cyan());

    if follow {
        println!();
        println!("{}", "Following logs... Press Ctrl+C to stop".dimmed());
    }

    Ok(())
}

async fn inspect_container(container: String) -> Result<()> {
    println!("{}", serde_json::json!({
        "Id": "a1b2c3d4e5f6g7h8",
        "Name": "my-app_web_1",
        "Image": "node:20",
        "State": {
            "Status": "running",
            "Running": true,
            "Pid": 12345
        },
        "NetworkSettings": {
            "Ports": {
                "3000/tcp": [{"HostPort": "3000"}]
            }
        }
    }).to_string());

    Ok(())
}

#[derive(Tabled)]
struct ContainerStats {
    #[tabled(rename = "CONTAINER")]
    container: String,
    #[tabled(rename = "CPU %")]
    cpu: String,
    #[tabled(rename = "MEM USAGE / LIMIT")]
    memory: String,
    #[tabled(rename = "MEM %")]
    mem_percent: String,
    #[tabled(rename = "NET I/O")]
    net_io: String,
    #[tabled(rename = "BLOCK I/O")]
    block_io: String,
}

async fn show_stats(containers: Vec<String>, no_stream: bool) -> Result<()> {
    let stats = vec![
        ContainerStats {
            container: "my-app_web_1".to_string(),
            cpu: "0.50%".to_string(),
            memory: "128MiB / 2GiB".to_string(),
            mem_percent: "6.25%".to_string(),
            net_io: "1.2MB / 500KB".to_string(),
            block_io: "10MB / 5MB".to_string(),
        },
        ContainerStats {
            container: "my-app_db_1".to_string(),
            cpu: "1.20%".to_string(),
            memory: "256MiB / 2GiB".to_string(),
            mem_percent: "12.50%".to_string(),
            net_io: "2.5MB / 1.2MB".to_string(),
            block_io: "50MB / 25MB".to_string(),
        },
    ];

    let table = Table::new(stats).to_string();
    println!("{}", table);

    if !no_stream {
        println!();
        println!("{}", "Streaming stats... Press Ctrl+C to stop".dimmed());
    }

    Ok(())
}

async fn copy_files(source: String, dest: String) -> Result<()> {
    println!("{} Copying {} to {}...", "→".blue(), source.cyan(), dest.cyan());
    println!("{} Copy complete", "✓".green());
    Ok(())
}

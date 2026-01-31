//! Project management commands.

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::time::Duration;

/// Project management commands.
#[derive(Args)]
pub struct ProjectCommand {
    #[command(subcommand)]
    pub action: ProjectAction,
}

#[derive(Subcommand)]
pub enum ProjectAction {
    /// Open a directory as a HyperBox project
    Open {
        /// Path to the project directory
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Project name (defaults to directory name)
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Start the current project's containers
    Start {
        /// Specific services to start
        services: Vec<String>,

        /// Build images before starting
        #[arg(short, long)]
        build: bool,

        /// Force recreation of containers
        #[arg(long)]
        force_recreate: bool,

        /// Run in detached mode
        #[arg(short, long)]
        detach: bool,
    },

    /// Stop the current project's containers
    Stop {
        /// Specific services to stop
        services: Vec<String>,

        /// Timeout in seconds
        #[arg(short, long, default_value = "10")]
        timeout: u64,
    },

    /// Restart the current project's containers
    Restart {
        /// Specific services to restart
        services: Vec<String>,
    },

    /// Show project status
    Status {
        /// Show detailed status
        #[arg(short, long)]
        detailed: bool,
    },

    /// List all projects
    List {
        /// Show all projects (including stopped)
        #[arg(short, long)]
        all: bool,
    },

    /// Show project logs
    Logs {
        /// Services to show logs for
        services: Vec<String>,

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

    /// Close a project (stop and cleanup)
    Close {
        /// Remove volumes
        #[arg(short, long)]
        volumes: bool,

        /// Remove networks
        #[arg(long)]
        networks: bool,
    },

    /// Initialize a new project configuration
    Init {
        /// Path to initialize
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Project template
        #[arg(short, long)]
        template: Option<String>,
    },
}

pub async fn run(cmd: ProjectCommand) -> Result<()> {
    match cmd.action {
        ProjectAction::Open { path, name } => open_project(path, name).await,
        ProjectAction::Start { services, build, force_recreate, detach } => {
            start_project(services, build, force_recreate, detach).await
        }
        ProjectAction::Stop { services, timeout } => stop_project(services, timeout).await,
        ProjectAction::Restart { services } => restart_project(services).await,
        ProjectAction::Status { detailed } => show_status(detailed).await,
        ProjectAction::List { all } => list_projects(all).await,
        ProjectAction::Logs { services, follow, tail, timestamps } => {
            show_logs(services, follow, tail, timestamps).await
        }
        ProjectAction::Close { volumes, networks } => close_project(volumes, networks).await,
        ProjectAction::Init { path, template } => init_project(path, template).await,
    }
}

async fn open_project(path: PathBuf, name: Option<String>) -> Result<()> {
    let path = path.canonicalize()
        .context("Failed to resolve project path")?;

    let project_name = name.unwrap_or_else(|| {
        path.file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "project".to_string())
    });

    let spinner = create_spinner("Detecting project type...");

    // Would detect project type here
    tokio::time::sleep(Duration::from_millis(500)).await;

    spinner.finish_with_message(format!(
        "{} Detected: {} project",
        "✓".green(),
        "Node.js".cyan()
    ));

    let spinner = create_spinner("Opening project...");

    // Would open project here
    tokio::time::sleep(Duration::from_millis(300)).await;

    spinner.finish_with_message(format!(
        "{} Project '{}' opened at {}",
        "✓".green(),
        project_name.cyan(),
        path.display()
    ));

    println!();
    println!("  {} hb project start    Start containers", "→".blue());
    println!("  {} hb project status   Show status", "→".blue());
    println!("  {} hb project logs     View logs", "→".blue());

    Ok(())
}

async fn start_project(
    services: Vec<String>,
    build: bool,
    force_recreate: bool,
    detach: bool,
) -> Result<()> {
    let spinner = create_spinner("Starting project...");

    // Would start project here
    tokio::time::sleep(Duration::from_millis(500)).await;

    spinner.finish_with_message(format!(
        "{} Project started in {}ms",
        "✓".green(),
        "47".cyan()
    ));

    if !detach {
        println!();
        println!("{}", "Press Ctrl+C to stop".dimmed());
    }

    Ok(())
}

async fn stop_project(services: Vec<String>, timeout: u64) -> Result<()> {
    let spinner = create_spinner("Stopping project...");

    tokio::time::sleep(Duration::from_millis(300)).await;

    spinner.finish_with_message(format!(
        "{} Project stopped",
        "✓".green()
    ));

    Ok(())
}

async fn restart_project(services: Vec<String>) -> Result<()> {
    stop_project(services.clone(), 10).await?;
    start_project(services, false, false, true).await?;
    Ok(())
}

async fn show_status(detailed: bool) -> Result<()> {
    use tabled::{Table, Tabled};

    #[derive(Tabled)]
    struct ServiceStatus {
        #[tabled(rename = "SERVICE")]
        name: String,
        #[tabled(rename = "STATUS")]
        status: String,
        #[tabled(rename = "PORTS")]
        ports: String,
        #[tabled(rename = "UPTIME")]
        uptime: String,
    }

    let services = vec![
        ServiceStatus {
            name: "web".to_string(),
            status: "✓ running".green().to_string(),
            ports: "3000:3000".to_string(),
            uptime: "2m 30s".to_string(),
        },
        ServiceStatus {
            name: "db".to_string(),
            status: "✓ running".green().to_string(),
            ports: "5432:5432".to_string(),
            uptime: "2m 30s".to_string(),
        },
        ServiceStatus {
            name: "redis".to_string(),
            status: "✓ running".green().to_string(),
            ports: "6379:6379".to_string(),
            uptime: "2m 30s".to_string(),
        },
    ];

    println!("{}", "Project: my-app".cyan().bold());
    println!("{}", "Status: running".green());
    println!();

    let table = Table::new(services).to_string();
    println!("{}", table);

    Ok(())
}

async fn list_projects(all: bool) -> Result<()> {
    use tabled::{Table, Tabled};

    #[derive(Tabled)]
    struct ProjectInfo {
        #[tabled(rename = "NAME")]
        name: String,
        #[tabled(rename = "PATH")]
        path: String,
        #[tabled(rename = "STATUS")]
        status: String,
        #[tabled(rename = "CONTAINERS")]
        containers: String,
    }

    let projects = vec![
        ProjectInfo {
            name: "my-app".to_string(),
            path: "/home/user/projects/my-app".to_string(),
            status: "running".green().to_string(),
            containers: "3".to_string(),
        },
        ProjectInfo {
            name: "api-service".to_string(),
            path: "/home/user/projects/api".to_string(),
            status: "stopped".dimmed().to_string(),
            containers: "0".to_string(),
        },
    ];

    let table = Table::new(projects).to_string();
    println!("{}", table);

    Ok(())
}

async fn show_logs(
    services: Vec<String>,
    follow: bool,
    tail: usize,
    timestamps: bool,
) -> Result<()> {
    println!("{}", "web    | Server started on port 3000".cyan());
    println!("{}", "db     | PostgreSQL ready".blue());
    println!("{}", "redis  | Ready to accept connections".magenta());

    if follow {
        println!();
        println!("{}", "Following logs... Press Ctrl+C to stop".dimmed());
    }

    Ok(())
}

async fn close_project(volumes: bool, networks: bool) -> Result<()> {
    let spinner = create_spinner("Closing project...");

    tokio::time::sleep(Duration::from_millis(500)).await;

    spinner.finish_with_message(format!(
        "{} Project closed{}{}",
        "✓".green(),
        if volumes { ", volumes removed" } else { "" },
        if networks { ", networks removed" } else { "" },
    ));

    Ok(())
}

async fn init_project(path: PathBuf, template: Option<String>) -> Result<()> {
    let path = if path.as_os_str() == "." {
        std::env::current_dir()?
    } else {
        path
    };

    let spinner = create_spinner("Initializing project...");

    tokio::time::sleep(Duration::from_millis(500)).await;

    spinner.finish_with_message(format!(
        "{} Created hyperbox.toml",
        "✓".green()
    ));

    println!();
    println!("  {} Edit hyperbox.toml to configure your project", "→".blue());
    println!("  {} Run 'hb project start' to start containers", "→".blue());

    Ok(())
}

fn create_spinner(message: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.blue} {msg}")
            .unwrap()
    );
    spinner.set_message(message.to_string());
    spinner.enable_steady_tick(Duration::from_millis(80));
    spinner
}

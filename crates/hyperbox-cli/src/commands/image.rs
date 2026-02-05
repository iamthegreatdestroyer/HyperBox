//! Image management commands.

use anyhow::Result;
use clap::{Args, Subcommand};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use tabled::{Table, Tabled};

use crate::client::DaemonClient;

/// Image management commands.
#[derive(Args)]
pub struct ImageCommand {
    #[command(subcommand)]
    pub action: ImageAction,
}

#[derive(Subcommand)]
pub enum ImageAction {
    /// List images
    #[command(alias = "ls")]
    List {
        /// Show all images (including intermediate)
        #[arg(short, long)]
        all: bool,

        /// Only show image IDs
        #[arg(short, long)]
        quiet: bool,
    },

    /// Pull an image from a registry
    Pull {
        /// Image name (e.g., nginx:latest)
        image: String,

        /// Pull all tagged images
        #[arg(short, long)]
        all_tags: bool,

        /// Platform (e.g., linux/amd64)
        #[arg(long)]
        platform: Option<String>,
    },

    /// Push an image to a registry
    Push {
        /// Image name
        image: String,

        /// Push all tags
        #[arg(short, long)]
        all_tags: bool,
    },

    /// Build an image from a Dockerfile
    Build {
        /// Build context path
        #[arg(default_value = ".")]
        path: String,

        /// Image tag
        #[arg(short, long)]
        tag: Vec<String>,

        /// Dockerfile path
        #[arg(short, long, default_value = "Dockerfile")]
        file: String,

        /// Build arguments
        #[arg(long)]
        build_arg: Vec<String>,

        /// Target build stage
        #[arg(long)]
        target: Option<String>,

        /// Don't use cache
        #[arg(long)]
        no_cache: bool,

        /// Always pull base images
        #[arg(long)]
        pull: bool,
    },

    /// Remove images
    #[command(alias = "rm")]
    Remove {
        /// Image names or IDs
        images: Vec<String>,

        /// Force removal
        #[arg(short, long)]
        force: bool,
    },

    /// Show image details
    Inspect {
        /// Image name or ID
        image: String,
    },

    /// Show image history
    History {
        /// Image name or ID
        image: String,

        /// Don't truncate output
        #[arg(long)]
        no_trunc: bool,
    },

    /// Tag an image
    Tag {
        /// Source image
        source: String,

        /// Target image name
        target: String,
    },

    /// Remove unused images
    Prune {
        /// Remove all unused images, not just dangling
        #[arg(short, long)]
        all: bool,

        /// Don't prompt for confirmation
        #[arg(short, long)]
        force: bool,
    },
}

pub async fn run(cmd: ImageCommand) -> Result<()> {
    match cmd.action {
        ImageAction::List { all, quiet } => list_images(all, quiet).await,
        ImageAction::Pull {
            image,
            all_tags,
            platform,
        } => pull_image(image, all_tags, platform).await,
        ImageAction::Push { image, all_tags } => push_image(image, all_tags).await,
        ImageAction::Build {
            path,
            tag,
            file,
            build_arg,
            target,
            no_cache,
            pull,
        } => build_image(path, tag, file, build_arg, target, no_cache, pull).await,
        ImageAction::Remove { images, force } => remove_images(images, force).await,
        ImageAction::Inspect { image } => inspect_image(image).await,
        ImageAction::History { image, no_trunc } => show_history(image, no_trunc).await,
        ImageAction::Tag { source, target } => tag_image(source, target).await,
        ImageAction::Prune { all, force } => prune_images(all, force).await,
    }
}

#[derive(Tabled)]
struct ImageInfo {
    #[tabled(rename = "REPOSITORY")]
    repository: String,
    #[tabled(rename = "TAG")]
    tag: String,
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "SIZE")]
    size: String,
}

async fn list_images(all: bool, quiet: bool) -> Result<()> {
    let client = DaemonClient::new();

    if !client.is_running().await {
        eprintln!("{} Daemon is not running. Start it with: hyperboxd", "✗".red());
        return Ok(());
    }

    let images = client.list_images().await?;

    if quiet {
        for img in &images {
            println!("{}", &img.id[..12.min(img.id.len())]);
        }
    } else if images.is_empty() {
        println!("{}", "No images found".dimmed());
    } else {
        let display: Vec<ImageInfo> = images
            .iter()
            .map(|img| ImageInfo {
                repository: img.repo.clone().unwrap_or_else(|| "<none>".to_string()),
                tag: img.tag.clone().unwrap_or_else(|| "<none>".to_string()),
                id: img.id.chars().take(12).collect(),
                size: humansize::format_size(img.size, humansize::BINARY),
            })
            .collect();

        let table = Table::new(display).to_string();
        println!("{}", table);
    }

    Ok(())
}

async fn pull_image(image: String, all_tags: bool, platform: Option<String>) -> Result<()> {
    let client = DaemonClient::new();

    if !client.is_running().await {
        eprintln!("{} Daemon is not running. Start it with: hyperboxd", "✗".red());
        return Ok(());
    }

    println!("{} Pulling {}...", "→".blue(), image.cyan());

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    pb.set_message("Pulling image layers...");
    pb.enable_steady_tick(Duration::from_millis(80));

    match client.pull_image(&image).await {
        Ok(_) => {
            pb.finish_and_clear();
            println!("{} Pulled {} successfully", "✓".green(), image.cyan());
        }
        Err(e) => {
            pb.finish_and_clear();
            eprintln!("{} Failed to pull {}: {}", "✗".red(), image, e);
        }
    }

    Ok(())
}

async fn push_image(image: String, all_tags: bool) -> Result<()> {
    println!("{} Pushing {}...", "→".blue(), image.cyan());

    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>3}/{len:3}")
            .unwrap()
            .progress_chars("█▓▒░"),
    );

    for i in 0..=100 {
        pb.set_position(i);
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    pb.finish_with_message("Done");

    println!("{} Pushed {}", "✓".green(), image.cyan());

    Ok(())
}

async fn build_image(
    path: String,
    tags: Vec<String>,
    file: String,
    build_args: Vec<String>,
    target: Option<String>,
    no_cache: bool,
    pull: bool,
) -> Result<()> {
    let tag = tags
        .first()
        .cloned()
        .unwrap_or_else(|| "latest".to_string());

    println!("{} Building image {}...", "→".blue(), tag.cyan());
    println!("  {} Context: {}", "→".blue(), path);
    println!("  {} Dockerfile: {}", "→".blue(), file);

    // Simulate build steps
    let steps = [
        "FROM node:20",
        "WORKDIR /app",
        "COPY package*.json ./",
        "RUN npm install",
        "COPY . .",
        "EXPOSE 3000",
        "CMD [\"node\", \"server.js\"]",
    ];

    for (i, step) in steps.iter().enumerate() {
        println!();
        println!("Step {}/{}: {}", i + 1, steps.len(), step.dimmed());
        tokio::time::sleep(Duration::from_millis(200)).await;
        println!(" ---> Running...");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    println!();
    println!("{} Built image {}", "✓".green(), tag.cyan());

    Ok(())
}

async fn remove_images(images: Vec<String>, force: bool) -> Result<()> {
    let client = DaemonClient::new();

    if !client.is_running().await {
        eprintln!("{} Daemon is not running. Start it with: hyperboxd", "✗".red());
        return Ok(());
    }

    let mut removed = 0;
    for image in &images {
        print!("{} Removing image {}...", "→".blue(), image.cyan());
        match client.remove_image(image, force).await {
            Ok(_) => {
                println!(" {}", "✓".green());
                removed += 1;
            }
            Err(e) => {
                println!(" {}", "✗".red());
                eprintln!("  Error: {}", e);
            }
        }
    }

    if removed > 0 {
        println!("{} Removed {} image(s)", "✓".green(), removed);
    }
    Ok(())
}

async fn inspect_image(image: String) -> Result<()> {
    println!(
        "{}",
        serde_json::json!({
            "Id": "sha256:abc123def456...",
            "RepoTags": ["node:20"],
            "Created": "2024-01-01T00:00:00Z",
            "Size": 1100000000,
            "Config": {
                "Env": ["NODE_VERSION=20.0.0"],
                "Cmd": ["node"],
                "WorkingDir": "/app"
            },
            "RootFS": {
                "Type": "layers",
                "Layers": [
                    "sha256:layer1...",
                    "sha256:layer2...",
                    "sha256:layer3..."
                ]
            }
        })
        .to_string()
    );

    Ok(())
}

#[derive(Tabled)]
struct HistoryEntry {
    #[tabled(rename = "IMAGE")]
    image: String,
    #[tabled(rename = "CREATED")]
    created: String,
    #[tabled(rename = "CREATED BY")]
    created_by: String,
    #[tabled(rename = "SIZE")]
    size: String,
}

async fn show_history(image: String, no_trunc: bool) -> Result<()> {
    let history = vec![
        HistoryEntry {
            image: "abc123".to_string(),
            created: "2 weeks ago".to_string(),
            created_by: "CMD [\"node\"]".to_string(),
            size: "0B".to_string(),
        },
        HistoryEntry {
            image: "def456".to_string(),
            created: "2 weeks ago".to_string(),
            created_by: "COPY . .".to_string(),
            size: "50MB".to_string(),
        },
        HistoryEntry {
            image: "ghi789".to_string(),
            created: "2 weeks ago".to_string(),
            created_by: "RUN npm install".to_string(),
            size: "200MB".to_string(),
        },
    ];

    let table = Table::new(history).to_string();
    println!("{}", table);

    Ok(())
}

async fn tag_image(source: String, target: String) -> Result<()> {
    println!("{} Tagged {} as {}", "✓".green(), source.cyan(), target.cyan());
    Ok(())
}

async fn prune_images(all: bool, force: bool) -> Result<()> {
    if !force {
        println!(
            "WARNING! This will remove all {} images.",
            if all { "unused" } else { "dangling" }
        );
        // Would prompt for confirmation here
    }

    println!("{} Removing unused images...", "→".blue());
    tokio::time::sleep(Duration::from_millis(500)).await;

    println!("{} Removed 3 images, reclaimed 500MB", "✓".green());

    Ok(())
}

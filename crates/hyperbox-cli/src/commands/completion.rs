//! Shell completion generation.

use anyhow::Result;
use clap::{Args, Command, CommandFactory};
use clap_complete::{generate, Generator, Shell};
use std::io;

/// Generate shell completions.
#[derive(Args)]
pub struct CompletionCommand {
    /// Shell to generate completions for
    #[arg(value_enum)]
    pub shell: Shell,
}

pub fn run(cmd: CompletionCommand) -> Result<()> {
    let mut app = super::Cli::command();
    print_completions(cmd.shell, &mut app);
    Ok(())
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

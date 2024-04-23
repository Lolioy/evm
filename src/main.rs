use clap::{Parser, Subcommand};

use crate::versions::*;

mod utils;
mod vars;
mod versions;

#[derive(Debug, Parser)]
#[command(version, author, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Go version manager
    Go {
        #[command(subcommand)]
        subcommand: Subcommands,
    },
}

#[derive(Debug, Subcommand)]
enum Subcommands {
    /// List versions
    #[command(visible_aliases = ["ls", "ll"])]
    List,
    /// List remote versions(default latest)
    #[command(visible_aliases = ["lr"])]
    ListRemote {
        /// List remote all versions(include archived versions)
        #[arg(short, long)]
        all: bool,
    },
    /// Use target version
    #[command(visible_aliases = ["u"])]
    Use { version: String },
    /// Install target version
    #[command(visible_aliases = ["in", "i"])]
    Install { version: String },
    /// Uninstall target version
    #[command(visible_aliases = ["un", "rm", "remove"])]
    Uninstall { versions: Vec<String> },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cmd = Args::parse().command;
    match cmd {
        Commands::Go { subcommand } => subcommand_match(go::Entry, subcommand).await?,
    };
    Ok(())
}

async fn subcommand_match<T: VersionOperator>(
    operator: T,
    subcmd: Subcommands,
) -> anyhow::Result<()> {
    // 匹配子命令
    match subcmd {
        Subcommands::List => operator.list_versions_local()?,
        Subcommands::ListRemote { all } => operator.list_versions_remote(all).await?,
        Subcommands::Use { version } => operator.use_version(version.as_str())?,
        Subcommands::Install { version } => operator.install_version(version.as_str()).await?,
        Subcommands::Uninstall { versions } => operator.uninstall_version(versions)?,
    };
    Ok(())
}

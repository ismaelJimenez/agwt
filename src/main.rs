use std::path::PathBuf;

use anstream::eprintln;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::CompleteEnv;
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};

use agwt::commands::{
    cd::cmd_cd,
    checkout::cmd_checkout,
    create::cmd_create,
    doctor::cmd_doctor,
    fetch::cmd_fetch,
    init::cmd_init,
    list::cmd_list,
    lock::{cmd_lock, cmd_unlock},
    move_wt::cmd_move,
    open::cmd_open,
    remove::{cmd_remove, cmd_remove_merged},
    shell_init::cmd_shell_init,
    sync::cmd_sync,
};
use agwt::git::resolve_bare_dir;
use agwt::{BOLD, RED};

/// Manage git worktrees backed by a bare repository.
///
/// The bare repo is discovered by looking for a `.bare` directory starting
/// from the current working directory and walking up. Override with `--bare-dir`.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Path to the bare repository directory.
    /// If omitted, auto-discovered from the working directory.
    #[arg(long, short = 'C', global = true)]
    bare_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all worktrees
    List,

    /// Create a worktree for a new branch (branched from default or --base)
    Create {
        /// Branch name to create
        branch: String,

        /// Directory name for the worktree (created as a sibling of the bare repo).
        /// If omitted, derived from the branch name (e.g. feature/xyz -> feature-xyz).
        #[arg(long)]
        name: Option<String>,

        /// Base ref (branch/tag/sha) to create the new branch from.
        /// If omitted, uses the repository's default branch.
        #[arg(long)]
        base: Option<String>,

        /// Remote name (default: origin)
        #[arg(long, default_value = "origin")]
        remote: String,
    },

    /// Create a worktree tracking an existing remote branch
    Checkout {
        /// Remote branch name to checkout
        branch: String,

        /// Directory name for the worktree (created as a sibling of the bare repo).
        /// If omitted, derived from the branch name (e.g. feature/xyz -> feature-xyz).
        #[arg(long)]
        name: Option<String>,

        /// Remote name (default: origin)
        #[arg(long, default_value = "origin")]
        remote: String,
    },

    /// Remove a worktree and its local branch
    Remove {
        /// Worktree directory name (required unless --merged is used)
        #[arg(add = ArgValueCompleter::new(complete_worktree_names))]
        name: Option<String>,

        /// Force removal even if worktree is dirty
        #[arg(long)]
        force: bool,

        /// Also delete the remote branch
        #[arg(long)]
        delete_remote: bool,

        /// Remove all worktrees whose branches are merged into the default branch
        #[arg(long)]
        merged: bool,

        /// Remote name (default: origin)
        #[arg(long, default_value = "origin")]
        remote: String,
    },

    /// Pull latest changes for a worktree
    Sync {
        /// Worktree directory name. If omitted, syncs the current directory
        /// if it is inside a worktree, otherwise fails.
        #[arg(add = ArgValueCompleter::new(complete_worktree_names))]
        name: Option<String>,

        /// Sync all worktrees at once
        #[arg(long)]
        all: bool,

        /// Remote name (default: origin)
        #[arg(long, default_value = "origin")]
        remote: String,
    },

    /// Fetch all remotes in the bare repo
    Fetch,

    /// Print the path to a worktree (use with shell function to cd)
    Cd {
        /// Worktree directory name
        #[arg(add = ArgValueCompleter::new(complete_worktree_names))]
        name: String,
    },

    /// Open a worktree in your editor ($VISUAL, $EDITOR, or code)
    Open {
        /// Worktree directory name
        #[arg(add = ArgValueCompleter::new(complete_worktree_names))]
        name: String,

        /// Editor command to use (overrides $VISUAL/$EDITOR)
        #[arg(long)]
        editor: Option<String>,
    },

    /// Move (rename) a worktree directory
    #[command(name = "move")]
    Move {
        /// Current worktree directory name
        #[arg(add = ArgValueCompleter::new(complete_worktree_names))]
        name: String,

        /// New directory name
        new_name: String,
    },

    /// Lock a worktree to prevent pruning
    Lock {
        /// Worktree directory name
        #[arg(add = ArgValueCompleter::new(complete_worktree_names))]
        name: String,

        /// Reason for locking
        #[arg(long)]
        reason: Option<String>,
    },

    /// Unlock a previously locked worktree
    Unlock {
        /// Worktree directory name
        #[arg(add = ArgValueCompleter::new(complete_worktree_names))]
        name: String,
    },

    /// Diagnose and fix worktree issues
    Doctor,

    /// Output shell configuration (eval this in your shell config)
    #[command(name = "shell-init")]
    ShellInit {
        /// Shell type
        #[arg(value_parser = ["bash", "zsh", "fish"])]
        shell: String,
    },

    /// Initialize a new bare repo for worktree management
    Init {
        /// Remote URL to clone from
        url: String,

        /// Name for the bare directory (default: derived from URL)
        #[arg(long)]
        name: Option<String>,
    },
}

fn main() {
    CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { url, name } => cmd_init(&url, name.as_deref()),
        Commands::ShellInit { shell } => cmd_shell_init(&shell),
        _ => {
            let bare_dir = match resolve_bare_dir(cli.bare_dir.as_deref()) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("{RED}error{RED:#}{BOLD}:{BOLD:#} {e:?}");
                    std::process::exit(1);
                }
            };
            match cli.command {
                Commands::List => cmd_list(&bare_dir),
                Commands::Create {
                    branch,
                    name,
                    base,
                    remote,
                } => {
                    let dir_name = name.unwrap_or_else(|| branch.replace('/', "-"));
                    cmd_create(&bare_dir, &dir_name, &branch, base.as_deref(), &remote)
                }
                Commands::Checkout {
                    branch,
                    name,
                    remote,
                } => {
                    let dir_name = name.unwrap_or_else(|| branch.replace('/', "-"));
                    cmd_checkout(&bare_dir, &dir_name, &branch, &remote)
                }
                Commands::Remove {
                    name,
                    force,
                    delete_remote,
                    merged,
                    remote,
                } => {
                    if merged {
                        cmd_remove_merged(&bare_dir, force, delete_remote, &remote)
                    } else {
                        let Some(name) = name else {
                            eprintln!(
                                "{RED}error{RED:#}{BOLD}:{BOLD:#} a worktree name is required (or use --merged)"
                            );
                            std::process::exit(1);
                        };
                        cmd_remove(&bare_dir, &name, force, delete_remote, &remote)
                    }
                }
                Commands::Sync { name, all, remote } => {
                    cmd_sync(&bare_dir, name.as_deref(), all, &remote)
                }
                Commands::Fetch => cmd_fetch(&bare_dir),
                Commands::Cd { name } => cmd_cd(&bare_dir, &name),
                Commands::Open { name, editor } => cmd_open(&bare_dir, &name, editor.as_deref()),
                Commands::Move { name, new_name } => cmd_move(&bare_dir, &name, &new_name),
                Commands::Lock { name, reason } => cmd_lock(&bare_dir, &name, reason.as_deref()),
                Commands::Unlock { name } => cmd_unlock(&bare_dir, &name),
                Commands::Doctor => cmd_doctor(&bare_dir),
                Commands::Init { .. } | Commands::ShellInit { .. } => unreachable!(),
            }
        }
    };

    if let Err(e) = result {
        eprintln!("{RED}error{RED:#}{BOLD}:{BOLD:#} {e:?}");
        std::process::exit(1);
    }
}

/// Complete worktree directory names for shell completion.
fn complete_worktree_names(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let mut candidates = Vec::new();
    let current_str = current.to_str().unwrap_or("");

    let Ok(cwd) = std::env::current_dir() else {
        return candidates;
    };

    let mut dir = cwd.as_path();
    let bare_dir = loop {
        let candidate = dir.join(".bare");
        if candidate.is_dir() && candidate.join("HEAD").exists() {
            break Some(candidate);
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => break None,
        }
    };

    let Some(bare_dir) = bare_dir else {
        return candidates;
    };

    let parent = bare_dir.parent().unwrap_or(&bare_dir);
    let Ok(entries) = std::fs::read_dir(parent) else {
        return candidates;
    };

    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        if !entry.path().join(".git").exists() {
            continue;
        }
        if name_str.starts_with(current_str) {
            candidates.push(CompletionCandidate::new(name_str.into_owned()));
        }
    }

    candidates
}

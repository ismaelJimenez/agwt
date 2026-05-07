# agwt

A CLI tool to manage git worktrees backed by a bare repository.

## Why agwt?

### vs. cloning the same repo multiple times

| | Multiple clones | agwt |
|---|---|---|
| Disk usage | Full `.git` per clone (objects, pack files, hooks) | Single `.bare/` shared by all worktrees |
| Fetching | Must `git fetch` in every clone independently | One `agwt fetch` updates everything |
| Branch state | Each clone has its own ref database — easy to lose track | All branches live in one place, `agwt list` shows them all |
| Creating a new workspace | Full network clone (slow for large repos) | Instant local checkout, no network needed |
| Consistency | Remotes/hooks/config can drift between clones | One config, one set of remotes, one hook directory |

### vs. raw `git worktree` commands

Raw `git worktree add/remove/list` works, but leaves several gaps that
become painful in daily use:

- **`git clone --bare` is broken out of the box** — it omits the fetch
  refspec, so `git fetch` silently does nothing. You must manually run
  `git config remote.origin.fetch "+refs/heads/*:refs/remotes/origin/*"`.
  agwt handles this on `init`.
- **No push setup** — new branches require `git push --set-upstream origin <branch>`
  every time. agwt sets `push.autoSetupRemote = true` so pushes just work.
- **Verbose commands** — creating a worktree with a new branch tracking a
  remote requires chaining `git fetch` + `git worktree add -b <branch> <path> <remote>/<base>`.
  agwt does it in one command: `agwt create feature/xyz`.
- **No status overview** — `git worktree list` shows paths and HEADs, but
  not dirty state, ahead/behind, or lock status. `agwt list` shows all of
  that at a glance.
- **Dangerous cleanup** — removing a worktree requires `git worktree remove`,
  then manually deleting the branch, then optionally pruning. agwt bundles
  removal + branch deletion + remote branch cleanup + prune in one step.
- **No bulk operations** — syncing or cleaning up multiple worktrees means
  writing shell loops. agwt provides `sync --all` and `remove --merged`.
- **No health checks** — stale worktrees, gone branches, and unpushed work
  silently accumulate. `agwt doctor` catches them.

### Ideal for AI-assisted development

Each worktree is a fully independent working directory. You can hand separate
worktrees to multiple AI coding agents running in parallel — each gets its
own branch and file state with zero interference.

## Installation

```bash
cargo install --path .
```

## Typical Setup

Create a dedicated development folder, initialize the bare repo inside it,
then add worktrees. This keeps everything self-contained and easy to ignore
from parent repos (just add the folder to `.gitignore`):

```bash
cd <your-repo>-dev
agwt init <your-repo>
cd <your-repo>
agwt checkout develop
agwt create feature/my-task --base develop
```

Resulting structure:

```
<your-repo>-dev/
└── <your-repo>/
    ├── .bare/           # bare repo (all git data)
    ├── develop/         # worktree tracking develop
    └── feature-my-task/ # worktree with new branch
```

Each worktree is an independent working directory — you can open them
in separate editor windows or feed them to AI coding agents in parallel.

`agwt list` shows the base branch when known:

```
  develop          develop
  feature-my-task  feature/my-task (from develop) [↑1]
```

## Quick Start

```bash
# Initialize a new bare repo from a remote URL
agwt init <your-repo>

# Create a worktree with a new branch (from default branch)
agwt create feature/xyz

# Create a worktree with a new branch from a specific base
agwt create fix/bug-123 --base develop

# Checkout a worktree tracking an existing remote branch
agwt checkout develop

# Checkout and record which branch it was based on
agwt checkout feature/xyz --base develop

# List all worktrees (shows dirty state, ahead/behind, lock status, base branch)
agwt list

# Sync (pull --rebase) a worktree by name
agwt sync feature-xyz

# Sync the current worktree (when inside one)
agwt sync

# Sync all worktrees at once
agwt sync --all

# Fetch all remotes
agwt fetch

# Remove a worktree and its local branch
agwt remove feature-xyz

# Remove and also delete the remote branch (asks for confirmation)
agwt remove feature-xyz --delete-remote

# Remove all worktrees whose branches are merged into the default branch
agwt remove --merged

# Move (rename) a worktree directory
agwt move old-name new-name

# Lock a worktree to prevent pruning
agwt lock feature-xyz --reason "on external drive"

# Unlock a worktree
agwt unlock feature-xyz

# Open a worktree in your editor ($VISUAL, $EDITOR, or code)
agwt open feature-xyz

# Open with a specific editor
agwt open feature-xyz --editor vim

# Change to a worktree directory
agwt cd feature-xyz

# Diagnose and fix worktree issues
agwt doctor
```

## Shell Setup

Add this to your shell config for `agwt cd` support and tab completions:

```bash
# bash (add to ~/.bashrc)
eval "$(agwt shell-init bash)"

# zsh (add to ~/.zshrc)
eval "$(agwt shell-init zsh)"
```

```fish
# fish (add to ~/.config/fish/config.fish)
agwt shell-init fish | source
```

## Bare Repo Discovery

The tool discovers the bare repository by searching for a `.bare` directory
starting from the current working directory and walking up. Override with
`--bare-dir` / `-C`.

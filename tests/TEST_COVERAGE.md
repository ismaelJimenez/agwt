# Test Coverage

## Commands & Options

| Command | Option | Test Coverage | Status |
|---------|--------|--------------|--------|
| **init** | (basic) | `init_creates_bare_dir` | ✅ |
| | `--name` | `init_custom_name` | ✅ |
| | name derived from URL | `init_derives_name_from_url` | ✅ |
| | dir already exists | `init_fails_if_dir_exists` | ✅ |
| | invalid URL | `init_fails_invalid_url` | ✅ |
| | git settings configured | `init_configures_git_settings` | ✅ |
| **list** | (empty) | `list_empty` | ✅ |
| | (with worktrees) | `list_shows_worktree` | ✅ |
| | (after remove) | `list_empty_after_remove` | ✅ |
| | dirty indicator | `list_shows_dirty_indicator` | ✅ |
| | ahead indicator | `list_shows_ahead_indicator` | ✅ |
| | behind indicator | `list_shows_behind_indicator` | ✅ |
| | locked indicator | `list_shows_locked_indicator` | ✅ |
| **create** | (basic/default base) | `create_default_base` | ✅ |
| | `--name` | `create_name_override` | ✅ |
| | `--base` | `create_with_base` | ✅ |
| | `--remote` | `create_with_remote` | ✅ |
| | slash→dash in dir name | `create_slash_to_dash` | ✅ |
| | dir already exists | `create_fails_if_dir_exists` | ✅ |
| | invalid base ref | `create_fails_invalid_base` | ✅ |
| **checkout** | (basic) | `checkout_existing_branch` | ✅ |
| | `--name` | `checkout_name_override` | ✅ |
| | `--remote` | `checkout_with_remote` | ✅ |
| | nonexistent branch | `checkout_fails_nonexistent_branch` | ✅ |
| | dir already exists | `checkout_fails_if_dir_exists` | ✅ |
| **remove** | (basic) | `remove_deletes_worktree_and_branch` | ✅ |
| | `--force` | `remove_force_dirty_worktree` | ✅ |
| | `--delete-remote` | `remove_delete_remote` | ✅ |
| | nonexistent worktree | `remove_fails_nonexistent` | ✅ |
| | `--merged` | `remove_merged_removes_only_merged_worktrees` | ✅ |
| | `--merged` (none merged) | `remove_merged_no_merged_worktrees` | ✅ |
| | no name or --merged | `remove_requires_name_or_merged` | ✅ |
| **sync** | (with name) | `sync_pulls_latest` | ✅ |
| | (auto-detect cwd) | `sync_auto_detect_cwd` | ✅ |
| | `--remote` | `sync_with_remote` | ✅ |
| | rebase (linear history) | `sync_rebase_linear_history` | ✅ |
| | rebase conflict | `sync_rebase_conflict` | ✅ |
| | `--all` | `sync_all` | ✅ |
| | `--all` (partial failure) | `sync_all_partial_failure` | ✅ |
| | `--all` (no worktrees) | `sync_all_empty` | ✅ |
| | autostash dirty worktree | `sync_autostash_dirty_worktree` | ✅ |
| | no remote branch | `sync_fails_no_remote_branch` | ✅ |
| **fetch** | (basic) | `fetch_works` | ✅ |
| **cd** | (basic) | `cd_outputs_path` | ✅ |
| | nonexistent worktree | `cd_fails_nonexistent` | ✅ |
| **open** | (basic) | `open_worktree` | ✅ |
| | nonexistent worktree | `open_fails_nonexistent` | ✅ |
| | `$EDITOR` env fallback | `open_uses_editor_env` | ✅ |
| **move** | (basic) | `move_worktree` | ✅ |
| | nonexistent source | `move_fails_nonexistent` | ✅ |
| | destination exists | `move_fails_dest_exists` | ✅ |
| **lock** | (basic + reason) | `lock_worktree` | ✅ |
| | nonexistent worktree | `lock_fails_nonexistent` | ✅ |
| **unlock** | (basic) | `unlock_worktree` | ✅ |
| **doctor** | healthy | `doctor_healthy` | ✅ |
| | no upstream | `doctor_no_upstream` | ✅ |
| | dirty worktree | `doctor_dirty` | ✅ |
| | ahead | `doctor_ahead` | ✅ |
| | behind | `doctor_behind` | ✅ |
| | gone branch | `doctor_gone_branch` | ✅ |
| | stale worktree | `doctor_stale_worktree` | ✅ |
| **shell-init** | bash | `shell_init_bash` | ✅ |
| | zsh | `shell_init_zsh` | ✅ |
| | fish | `shell_init_fish` | ✅ |
| | invalid shell | `shell_init_invalid_shell` | ✅ |
| **global** | `--bare-dir` (invalid) | `bare_dir_nonexistent_fails` | ✅ |
| | no bare-dir outside project | `no_bare_dir_outside_project_fails` | ✅ |
| | `--bare-dir` (valid) | all tests via `agwt()` helper | ✅ |
| | `--version` | `version_flag` | ✅ |
| **lifecycle** | full workflow | `full_workflow` | ✅ |

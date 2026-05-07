use std::io::Write;
use std::path::Path;

use anyhow::Result;

use crate::git::{list_worktrees, parent_of_bare};
use crate::{BOLD, GREEN, RED, YELLOW};

pub fn cmd_list(bare_dir: &Path) -> Result<()> {
    let parent = parent_of_bare(bare_dir);
    let entries = list_worktrees(bare_dir, &parent)?;

    if entries.is_empty() {
        println!("No worktrees");
        return Ok(());
    }

    let max_name = entries.iter().map(|e| e.name.len()).max().unwrap_or(0);

    let mut stdout = anstream::stdout().lock();
    for entry in &entries {
        let mut indicators = Vec::new();

        if entry.dirty {
            indicators.push(format!("{RED}*{RED:#}"));
        }
        if entry.ahead > 0 || entry.behind > 0 {
            let ab = match (entry.ahead, entry.behind) {
                (a, 0) => format!("{YELLOW}↑{a}{YELLOW:#}"),
                (0, b) => format!("{YELLOW}↓{b}{YELLOW:#}"),
                (a, b) => format!("{YELLOW}↑{a}↓{b}{YELLOW:#}"),
            };
            indicators.push(ab);
        }
        if entry.locked {
            indicators.push(format!("{RED}locked{RED:#}"));
        }

        let status = if indicators.is_empty() {
            String::new()
        } else {
            format!(" [{}]", indicators.join(" "))
        };

        let base_info = match &entry.base {
            Some(b) => format!(" (from {BOLD}{b}{BOLD:#})"),
            None => String::new(),
        };

        writeln!(
            stdout,
            "  {GREEN}{:<width$}{GREEN:#}  {branch}{base_info}{status}",
            entry.name,
            branch = entry.branch,
            width = max_name,
        )?;
    }

    Ok(())
}

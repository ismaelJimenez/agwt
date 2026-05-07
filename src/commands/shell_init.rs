use anyhow::Result;

pub fn cmd_shell_init(shell: &str) -> Result<()> {
    let output = match shell {
        "bash" => {
            r#"agwt() {
  if [[ "$1" == "cd" ]]; then
    local dir
    dir=$(command agwt "$@") && cd "$dir"
  else
    command agwt "$@"
  fi
}
source <(COMPLETE=bash agwt)"#
        }
        "zsh" => {
            r#"agwt() {
  if [[ "$1" == "cd" ]]; then
    local dir
    dir=$(command agwt "$@") && cd "$dir"
  else
    command agwt "$@"
  fi
}
source <(COMPLETE=zsh agwt)"#
        }
        "fish" => {
            r#"function agwt
  if test "$argv[1]" = "cd"
    set -l dir (command agwt $argv)
    and cd $dir
  else
    command agwt $argv
  end
end
COMPLETE=fish agwt | source"#
        }
        "powershell" => {
            r#"function agwt {
    if ($args[0] -eq 'cd') {
        $dir = & agwt.exe @args
        if ($LASTEXITCODE -eq 0) { Set-Location $dir }
    } else {
        & agwt.exe @args
    }
}
Register-ArgumentCompleter -Native -CommandName agwt -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)
    $env:COMPLETE = 'powershell'
    agwt.exe | ForEach-Object {
        [System.Management.Automation.CompletionResult]::new($_)
    }
    Remove-Item Env:\COMPLETE
}"#
        }
        _ => unreachable!(),
    };
    println!("{output}");
    Ok(())
}

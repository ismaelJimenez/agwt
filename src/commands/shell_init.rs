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
        _ => unreachable!(),
    };
    println!("{output}");
    Ok(())
}

# CLAUDE.md

## Completion criteria

Before any feature or fix is considered done, **all** of the following must pass:

```sh
cargo test
cargo fmt -- --check
cargo clippy -- -D warnings
```

Do not mark work as complete until all three commands exit successfully.

# CLAUDE.md

## Completion criteria

Before any feature or fix is considered done, **all** of the following must be true:

1. There must be a test (or tests) proving the new behaviour or fix.
2. All of the following commands must pass:

```sh
cargo test
cargo fmt -- --check
cargo clippy -- -D warnings
```

Do not mark work as complete until a covering test exists and all three commands exit successfully.

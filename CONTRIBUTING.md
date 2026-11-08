# Contributing

Thanks for helping improve Ocypus Iota A40 for Linux. Keep changes focused,
document user-visible behavior, and avoid committing device-specific output or
local reverse-engineering artifacts.

## Before opening a change

From the repository root, run:

```sh
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

Changes to device I/O or temperature handling should include tests where
practical. Hardware checks should be documented separately from automated
checks so contributors can distinguish verified behavior from assumptions.

## Pull requests

- Explain the problem and the behavior changed.
- Include the checks you ran and note any hardware-only checks.
- Keep unrelated formatting or generated files out of the change.
- Do not commit `target/`, files under `analysis/`, or the vendor installer.

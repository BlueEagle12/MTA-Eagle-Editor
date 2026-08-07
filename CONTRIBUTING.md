# Contributing to Eagle Editor

Thanks for helping improve Eagle Editor. Bug reports, documentation fixes, and
focused pull requests are welcome.

## Development setup

1. Install Rust 1.85 or newer through [rustup](https://rustup.rs/).
2. Clone the repository.
3. Run `cargo test --locked`.
4. Start the editor with `cargo run --release -- /path/to/resource`.

Linux development requires an X11 desktop, OpenGL drivers, and the usual native
compiler toolchain. Optional `.blend` import support also requires Blender 4.2+
and the add-ons described in the main README.

## Before opening a pull request

Run these checks from the repository root:

```bash
cargo fmt --all -- --check
cargo test --locked
```

Keep filesystem and CPU-heavy asset operations off the render/input thread.
Background jobs should snapshot thread-safe project data, report progress over
channels, and apply completed results in bounded per-frame batches.

Project-owned structured metadata belongs in a versioned section of the
resource-root `EagleScene.eaglescne` file. Do not add new editor sidecars unless
an external format requires one.

## Reporting issues

Include the operating system, reproduction steps, expected result, actual
result, and a small test resource when licensing permits. Do not attach files
from a commercial GTA: San Andreas installation.


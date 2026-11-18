# Ironshade

Ironshade is a pure-Rust shader compiler that aims to compile the Slang™ shading
language to SPIR-V, WGSL, GLSL ES, HLSL and legacy Direct3D bytecode.

The repository currently contains the workspace scaffold and dependency checks.
Shader compilation, reflection, and the oracle harness are milestones described
in the [architecture plan](docs/IRONSHADE_ARCHITECTURE.md).

## Getting started

Install [Rust with rustup](https://rustup.rs/) and run these commands from the
repository root. Rustup selects the toolchain pinned by the workspace.

```sh
cargo build --locked --workspace
cargo run --locked -p ironshade --bin ironshadec -- --help
cargo xtask check-layering
```

Compiler packages live in `crates/`, developer tools in `tools/`, and sample
integrations in `examples/`. See [CONTRIBUTING.md](CONTRIBUTING.md) for the build
matrix and [AGENTS.md](AGENTS.md) for crate boundaries.

Ironshade is licensed under either [MIT](LICENSE-MIT) or
[Apache 2.0](LICENSE-APACHE), at your option.

Slang™ and the Slang logo are trademarks of the Khronos Group Inc.

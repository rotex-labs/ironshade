<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/brand/ironshade-dark.svg">
    <img alt="Ironshade" src="docs/brand/ironshade-light.svg" width="1200">
  </picture>
</p>

# Ironshade

Ironshade is a pure-Rust shader compiler that aims to compile the Slang™ shading
language to SPIR-V, WGSL, GLSL ES, HLSL and legacy Direct3D bytecode.

> [!NOTE]
> **Early development.** M0 establishes the compiler contracts and shader
> oracle harness. Ironshade shader emission starts in M1.

## Overview

Ironshade is designed for shader authors and engine developers who need multiple
graphics targets and a consistent view of shader bindings and layouts. Shader
source and typed shader graphs will lower into the same intermediate
representation, Ironshade Graph IR. A versioned backend HAL lets developers add
targets in their own Rust crates.

Each backend declares the capabilities of its target profile. Legalization
lowers operations through shared recipes and target-specific passes, then checks
the result before emission. Requirements that cannot be met produce diagnostics.
Emulation requires explicit compile policy approval. Artifacts retain entry
points, bindings, layouts, and remaps for the host application.

Ironshade is developed in the Rotex ecosystem and builds as a standalone Cargo
workspace.

## Key crates

| Crate | Role |
| --- | --- |
| [`ironshade`](crates/ironshade) | Umbrella library and `ironshadec` CLI |
| [`ironshade-ir`](crates/ironshade-ir) | Shared representation for text and graph input |
| [`ironshade-caps`](crates/ironshade-caps) | Capability vocabulary and legalization planning |
| [`ironshade-target`](crates/ironshade-target) | Public backend HAL |
| [`ironshade-artifact`](crates/ironshade-artifact) | Versioned reflection and artifact format |
| [`ironshade-runtime`](crates/ironshade-runtime) | Planned live compilation service |

## Target roadmap

| Output | Environment | Planned milestone |
| --- | --- | --- |
| SPIR-V | Vulkan | M1 |
| WGSL | WebGPU | M1 |
| GLSL ES 3.00 | WebGL2 | M3 |
| HLSL source | Direct3D | M3 |
| DXIL via DXC | Direct3D 12 | M4 |
| DXBC | Direct3D 10 and 11 | M5 |
| Shader Model 2 and 3 tokens | Direct3D 9 | M6 |
| Native DXIL | Direct3D 12 | M7 |

See the [architecture plan](docs/IRONSHADE_ARCHITECTURE.md) for milestone exit
criteria and the [language roadmap](docs/language-subset.md) for subset status.

## Getting started

Install [Rust with rustup](https://rustup.rs/) and run from the repository root.

```sh
cargo build --locked --workspace
cargo run --locked -p ironshade --bin ironshadec -- --help
cargo xtask check-layering
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the local checks and
[AGENTS.md](AGENTS.md) for dependency rules.

## License

Licensed under either the [Apache License, Version 2.0](LICENSE-APACHE) or
the [MIT license](LICENSE-MIT), at your option. See [NOTICE](NOTICE) for
third-party attributions.

Slang™ and the Slang logo are trademarks of the Khronos Group Inc.

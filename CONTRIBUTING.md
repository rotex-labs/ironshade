# Contributing to Ironshade

Ironshade is a standalone Cargo workspace. Compiler libraries and the
`ironshadec` binary live in `crates/`. Differential testing, conformance tooling,
and reduction live in `tools/`. The `xtask` package provides repository commands.
The [architecture plan](docs/IRONSHADE_ARCHITECTURE.md) describes the planned
contracts and milestones. [AGENTS.md](AGENTS.md) summarizes dependency rules.

## Local checks

Rustup reads the pinned development toolchain from `rust-toolchain.toml`. The
minimum supported Rust version is 1.90.0. Commit `Cargo.lock` changes with
workspace dependency changes.

```sh
cargo fmt --all -- --check
cargo build --locked --workspace
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
cargo xtask check-layering
cargo deny --locked --all-features check
```

The dependency audit uses [cargo-deny](https://github.com/EmbarkStudios/cargo-deny).
To reproduce the minimum-version and WebAssembly checks, install the respective
toolchain and target.

```sh
rustup toolchain install 1.90.0 --profile minimal
cargo +1.90.0 check --locked --workspace --all-targets --all-features
rustup target add wasm32-unknown-unknown
cargo build --locked --target wasm32-unknown-unknown -p ironshade --no-default-features --lib
cargo build --locked --target wasm32-unknown-unknown -p ironshade-runtime
```

## Oracle checks

Run the Nova baseline on Linux x86-64 with Git, curl, tar, CMake, Ninja, Python 3,
and a C++ toolchain available. The first run provisions pinned tools in
`target/ironshade-oracles/`.

```sh
git submodule update --init --depth 1 conformance/slang-tests
cargo xtask conformance --dry-run
cargo xtask fetch-oracles
cargo xtask difftest --oracle-only tests/fixtures/rotex
```

Use `--update` on the difftest command for deliberate snapshot revisions. Review
the normalized JSON and the raw evidence before accepting a change. For the
baseline format and validator results, see [Differential testing](docs/difftest.md).

## Branches and releases

Open feature pull requests against `main`. Use conventional commits for
release notes, with a crate scope where it makes the change clearer.

The compiler packages share one workspace version and one release-plz version
group. The first release follows M0, once the oracle harness and initial public
contracts are usable. Activate publication then using the manual release flow
used by the Rotex crate repositories.

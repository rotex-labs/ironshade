# Slang compiler test inventory

`slang-tests/` is a submodule pinned to Slang `v2026.21`. The
[manifest](manifest.toml) records the revision, oracle pins, and reviewed
licenses. The license audit checks the pinned revision, the set of license
files, and their hashes.

```sh
git submodule update --init --depth 1 conformance/slang-tests
cargo xtask conformance --audit
cargo xtask conformance --dry-run
```

The dry run audits the suite and writes `report.json` and `index.html` under
`target/ironshade-conformance/`. Counts cover directive types, categories, and
applicability to compile-only comparison. The report is an inventory. Coverage
measurements start when Ironshade can compile and compare the applicable cases.

See the [differential harness](../docs/difftest.md).

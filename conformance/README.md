# Conformance harness

M0 will pin the Slang compiler test suite as a submodule at `slang-tests/`. Its
`manifest.toml` will record the revision, verified oracle checksums, and
directory-level license decisions. `features.toml` will map tests to feature
areas. `expected/` will hold reviewed differences, and `dashboard/` will hold
generated reports.

See the [harness design](../docs/difftest.md).

# Differential testing

M0 records a slangc 2026.21 baseline for the Nova shaders. The snapshot records
the target, compiler flags, target validation result, and normalized entry
points, bindings, offsets, and layouts. Logical coordinates come from the
oracle reflection JSON. Physical coordinates that it does not report remain
unknown.

## Running the Nova baseline

On Linux x86-64, provision the pinned tools and verify the checked-in snapshots.

```sh
cargo xtask fetch-oracles
cargo xtask difftest --oracle-only tests/fixtures/rotex
```

There are 18 cases across nine entry points and two targets. The fixtures and
job definitions live in [tests/fixtures/rotex](../tests/fixtures/rotex/).
The emitted payloads, raw Slang reflection, logs, and normalized JSON are kept
under `target/ironshade-difftest/`. Use `--update` after reviewing a deliberate
baseline change.

The baseline retains validator rejections as data. slangc 2026.21 emits
`var<immediate>` in the WGSL for three Nova cases, and the pinned Naga validator
rejects those payloads. The snapshots record those results alongside the
logical push-constant entries reported by slangc.

## Classifying the Slang compiler test suite

The [conformance inventory](../conformance/) checks the pinned test suite and
its licenses, then reports the upstream directives by command, category,
and applicability. It classifies the test arguments without running the
upstream test commands.

See the [differential testing design](IRONSHADE_ARCHITECTURE.md#10-checked-against-slangc-differential-and-conformance-harness)
for the planned compiler comparisons, verdicts, and coverage calculation.

Slang™ and the Slang logo are trademarks of the Khronos Group Inc.

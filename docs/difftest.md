# Differential testing

M0 will use slangc 2026.21 to establish normalized reflection snapshots for the
Nova shaders. The harness will compare semantic fields such as entry points,
bindings, offsets, and strides. It will record validator results and
diagnostic classes. An expected difference will carry a reason and a tracking
issue.

The [differential testing design](IRONSHADE_ARCHITECTURE.md#10-checked-against-slangc-differential-and-conformance-harness)
defines the planned pins, verdicts, and coverage calculation.

Slang™ and the Slang logo are trademarks of the Khronos Group Inc.

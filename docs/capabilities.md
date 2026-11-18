# Capabilities

`ironshade-caps` defines the initial capability vocabulary. A profile can
satisfy a requirement natively, with a lowering recipe, or with an explicitly
approved emulation recipe. Missing declarations resolve to Unknown.
Unsupported or unknown requirements produce diagnostics.

The HAL schedules shared recipes before target passes and records each
resolution in the finished artifact. See the
[capability model](IRONSHADE_ARCHITECTURE.md#7-capability-model-ironshade-caps)
for the planned vocabulary and target limits.

Slang™ and the Slang logo are trademarks of the Khronos Group Inc.

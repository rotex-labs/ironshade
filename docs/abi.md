# ABI and reflection

`ironshade-artifact` defines schema v0 for versioned JSON reflection and shader
artifacts. It records entry points, stage interfaces, declared and live
bindings, layout measurements, remaps, target identity, and capability reports.
Measurements that the producer does not supply remain unknown.

The Nova snapshots exercise the schema using slangc reflection. Ironshade's
layout planner and target-specific layout probes are part of later milestones.

See the [ABI and reflection design](IRONSHADE_ARCHITECTURE.md#8-abi-and-reflection)
for the planned layout rules.

Slang™ and the Slang logo are trademarks of the Khronos Group Inc.

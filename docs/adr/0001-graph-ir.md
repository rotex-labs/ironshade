# ADR 0001 — Ironshade Graph IR

## Context

Text and typed shader graphs need a shared semantic model. Shader targets also
need structured control flow and explicit ordering of side effects.

## Decision

Use a regionalized value and state dependence graph. Branches, loops,
functions, and globals have explicit regions. State edges order effectful
operations, and rewrites preserve provenance for diagnostics.

## Consequences

Frontends lower to a common builder and verifier. Backends emit structured
control flow from the region tree. Rewrites account for both value and state
dependencies. See the [IR design](../IRONSHADE_ARCHITECTURE.md#5-ir-ironshade-graph-ir-igir).

Slang™ and the Slang logo are trademarks of the Khronos Group Inc.

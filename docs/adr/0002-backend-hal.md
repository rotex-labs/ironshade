# ADR 0002 — Backend HAL

## Context

Backends have different profiles, capability limits, legalization requirements,
and output formats. Third-party backends need a public interface.

## Decision

Expose a versioned HAL in `ironshade-target`. A target declares its capabilities
and provides legalization, ABI planning, and emission. Shared artifact
construction packages the payload with reflection. Static dispatch is the
default, and an optional registry provides dynamic selection.

## Consequences

Targets can live in separate crates and be selected with Cargo features. The
compiler applies the same capability and reflection contracts across backends.
See the [HAL design](../IRONSHADE_ARCHITECTURE.md#6-backend-hal-ironshade-target).

Slang™ and the Slang logo are trademarks of the Khronos Group Inc.

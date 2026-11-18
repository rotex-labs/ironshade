# Ironshade development

- Read the [architecture plan](docs/IRONSHADE_ARCHITECTURE.md) for package roles,
  contracts, and milestone boundaries. The [README](README.md) tracks current
  implementation status.
- Keep Ironshade independent of Rotex. Adapters live in Rotex. Test fixtures
  drawn from Rotex carry source paths and revision information.
- Both text and graph frontends lower to Ironshade Graph IR and stay independent
  of each other. Backends use the public HAL and keep frontend dependencies out
  of their dependency graphs.
- Keep `ironshade-artifact` dependency-light. Its direct dependencies are
  limited to `ironshade-base` and `serde`. Compiler packages cannot depend on
  repository tools.
- Preserve deterministic output and lossless reflection. Unsupported or unknown
  capabilities produce diagnostics. Emulation requires explicit approval from
  the compile policy.
- Use the shared formatter and run the checks in
  [CONTRIBUTING.md](CONTRIBUTING.md).

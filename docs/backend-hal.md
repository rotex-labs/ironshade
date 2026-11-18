# Backend HAL

`ironshade-target` provides the public static backend interface. A target
declares its profiles and capabilities, legalizes a module for a profile, and
emits a source or binary payload. The pipeline checks target legality before
physical ABI planning and checks the payload before it attaches reflection.

M0 exposes an empty IGIR module and an external example backend that exercises
the contract. The first shader targets land in M1.

See [ADR 0002](adr/0002-backend-hal.md) and the
[HAL design](IRONSHADE_ARCHITECTURE.md#6-backend-hal-ironshade-target). The
[external backend package](../examples/external-backend/) uses the HAL as its
sole dependency.

Slang™ and the Slang logo are trademarks of the Khronos Group Inc.

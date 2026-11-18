# Backend HAL

`ironshade-target` is the home of the public backend interface. A backend will
declare its profiles and capabilities, legalize input for a selected profile,
and emit a source or binary payload. Shared artifact construction will attach
reflection to that payload.

See [ADR 0002](adr/0002-backend-hal.md) and the
[HAL design](IRONSHADE_ARCHITECTURE.md#6-backend-hal-ironshade-target). The
[external backend package](../examples/external-backend/) checks that the
example can build with the HAL as its sole dependency.

Slang™ and the Slang logo are trademarks of the Khronos Group Inc.

# Fuzzing

This directory will hold the nightly cargo-fuzz workspace. Add targets under
`fuzz_targets/` as the lexer, parser, semantic analysis, IR reader, graph and
artifact decoders, and SPIR-V emission become available. Keep seed inputs in
`corpus/`.

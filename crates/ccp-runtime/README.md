# CCP constructive interpreter

Interprets the [CCP specifications](../../ARCHITECTURE.md) with private communication-DAG and proof-of-work
carriers. `Machine` executes typed inputs through `HistoryExpansion` and delegates observations to
`HistoryCollapse`. Protocol meanings remain in their owning specs' generic extensions. Dependencies name those
specs directly; the runtime does not depend on the `ccp` facade.

Run the executable comparisons with `cargo run -p consensus-ccp-examples`.

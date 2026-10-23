# CCP executable comparisons

Runs protocol comparisons over the [CCP specifications](../../ARCHITECTURE.md) through the constructive machine in `consensus-ccp-runtime`. Each example builds histories only from typed inputs and reads them only through collapse.

- `policies` compares candidate, horizon, and leader policies over one communication history.
- `equivocations` contrasts how protocols treat a retained equivocation.
- `nakamoto` shows proof-of-work expansion, greatest-work fork choice, and confirmation-depth collapse.

Run one with `cargo run -p consensus-ccp-examples -- policies`, or all with `cargo run -p consensus-ccp-examples`.

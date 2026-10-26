# CCP

**Consensus as expansion and collapse over an explicit semantic carrier.**

The `consensus-ccp` package groups the independent `ccp-*` specifications under one path. The core
vocabulary sits at the root; `ccp::validator`, `ccp::quorum`, `ccp::communication`, `ccp::leader`, and `ccp::confidence`
name the shared specifications, and `ccp::protocols` names each protocol specification. Its Rust
library name is `ccp`.

Applications may use the facade; interpreters and other specs depend directly on the capabilities
they consume. `ccp-core` names candidate and collapse sorts. `ccp-validator`, `ccp-quorum`, `ccp-communication`,
`ccp-leader`, and `ccp-confidence` supply shared meanings. Protocol crates compose these meanings;
laws are exported by the specification that owns them.

The facade contains no interpreter and no scenario material. Concrete machine memory lives in
`consensus-ccp-runtime` under `crates/ccp-runtime`, conformance runners in `crates/ccp-tests`, and
scenarios in `ccp-scenarios` and `ccp-comparison`.
The full paper and architecture live in the [CCP repository](https://github.com/tgrospic/consensus-ccp).

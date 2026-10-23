# ccp-quorum

Validator vocabulary, membership, committee order, attributed evidence, and quorum arithmetic.

The public surface includes `ValidatorSorts`, `ValidatorSet`, `CommitteeOrder`, `WitnessSelection`, `QuorumOf`.

Witness selection, support relation, and threshold compose independently. This spec depends only on `ccp-core` among the domain specs.

Concrete interpreters live under `crates/`. Portable obligations live with the capabilities they constrain.

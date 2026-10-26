# ccp-quorum

Attributed validator evidence, witness selection, and weighted quorum composition over the `ccp-validator` committee.

The public surface includes `ValidatorEvidence`, `CandidateSupport`, `WitnessSelection`, `QuorumOf`, and the derived quorum and legitimacy extensions.

Witness selection, support relation, and threshold compose independently. A quorum is the weight of distinct supporting committee members out of the total committee weight. This spec depends only on `ccp-core` and `ccp-validator`.

Concrete interpreters live under `crates/`. Portable committee and threshold obligations live in `ccp-validator`.

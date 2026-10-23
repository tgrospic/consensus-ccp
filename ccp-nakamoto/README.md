# ccp-nakamoto

Work ancestry, cumulative-work fork choice, confirmation prefixes, and separate reversal risk.

The public surface includes `BlockParent`, `ProofOfWork`, `NakamotoCollapseExt`.

`BlockParent` and `ProofOfWork` inherit the same `CandidateSorts`. This spec depends on `ccp-core` and `ccp-confidence`, with no Byzantine quorum or round-DAG dependency.

Concrete interpreters live under `crates/`. Portable obligations live with the capabilities they constrain.

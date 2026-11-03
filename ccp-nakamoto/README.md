# ccp-nakamoto

Work ancestry, cumulative-work fork choice, confirmation prefixes, and separate reversal risk.

The public surface includes `BlockParent`, `ProofOfWork`, `NakamotoCollapseExt`.

`BlockParent` and `ProofOfWork` inherit the same `CandidateSorts`. This spec depends on `ccp-core` and `ccp-confidence`, with no Byzantine quorum or round-DAG dependency.

Nakamoto collapse does not satisfy the deterministic agreement of paper Definition 2.6: an accepted prefix is final only against less work, and a heavier fork reverses it. The conformance tests witness both sides: extending the heaviest chain keeps every accepted block, and a heavier fork removes one. How unlikely that fork is belongs to the separate risk model (`AdversarialHashFraction`), not to the collapse.

Concrete interpreters live under `crates/`. Portable obligations live with the capabilities they constrain.

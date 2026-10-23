# ccp-dense-leader

Unbounded legitimacy-based leader collapse.

The public surface includes `SlotLeader`, `DenseLeaderEvidenceCollapseExt`.

Composes the shared quorum and communication meanings with explicit leader selection. Laws are exported here; single-protocol scenarios live in `ccp-scenarios` and scheduled comparisons in `ccp-comparison`.

Concrete interpreters live under `crates/`. Portable obligations live with the capabilities they constrain.

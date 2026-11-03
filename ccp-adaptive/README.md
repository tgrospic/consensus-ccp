# ccp-adaptive

Adaptive evidence horizons, certified predecessor repair, and justification-history fringes.

The public surface includes `AdaptiveEvidence`, `CertifiedPredecessor`, `AdaptiveCollapseExt`.

Composes core, communication, quorum, and leader capabilities. `AdaptiveDagView` in `ccp-runtime` interprets it from one observer's communication history, and the conformance runners check `check_adaptive_anchor_chain`, `check_adaptive_direct_legitimacy`, and `check_adaptive_prefix_compatibility` from every observer above every frontier of each Mysticeti and scheduled-leader scenario.

TODO: add an Adaptive `HistoryCollapse` so collapse agreement and handover continuity run on it; its decisions are relative to a finalized frontier, which the collapse query has to carry.

Concrete interpreters live under `crates/`. Portable obligations live with the capabilities they constrain.

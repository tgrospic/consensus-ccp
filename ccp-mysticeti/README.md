# ccp-mysticeti

Mysticeti direct commit and skip evidence, ordered anchor repair, and prefix closure.

The public surface includes `MysticetiDagEvidence`, `MysticetiDirectViewExt`, `MysticetiCollapseViewExt`.

Composes core, communication, quorum, and leader capabilities. Candidate observations and repair use the same `CandidateSorts`. Laws are exported here; Mysticeti scenarios live in `ccp-scenarios` and cross-protocol comparisons in `ccp-comparison`.

Concrete interpreters live under `crates/`. Portable obligations live with the capabilities they constrain.

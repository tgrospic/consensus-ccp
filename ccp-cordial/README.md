# ccp-cordial

Cordial approval, ratification, final leaders, predecessor chains, and ordered fragments.

The public surface includes `CordialEvidenceExt`, `CordialCollapseExt`, `CordialOrderExt`.

Composes core, communication, quorum, and leader capabilities without depending on Mysticeti. Candidate and predecessor operations share `CandidateSorts`. Laws are exported here; Cordial scenarios live in `ccp-scenarios`.

Concrete interpreters live under `crates/`. Portable obligations live with the capabilities they constrain.

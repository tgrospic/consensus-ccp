# ccp-confidence

Protocol-distinct confidence evidence and acceptance thresholds.

The public surface includes `ConfidenceEvidence`, `AcceptanceThreshold`, `ConfidenceAcceptanceExt`.

Confidence kinds remain distinct: confirmation depth and sampling evidence do not become one interchangeable score. This spec has no dependency on a particular consensus protocol.

Concrete interpreters live under `crates/`. Portable obligations live with the capabilities they constrain.

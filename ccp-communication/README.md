# ccp-communication

The Byzantine message carrier: message vocabulary, causal justification, round advancement, admission of new messages, and the evidence observed from the carrier.

The specification surface includes `CommunicationSorts`, `MessageCarrier`, `MessageJustification`, `AdvancePolicy`, `MessageInput`, `ExpansionError`, `MessageAdmissionExt`, `PreviousRoundWitnesses`, and the `MessageHistoryLawsExt`, `MessageExpansionLawsExt`, and `JustificationLawsExt` laws. `JustificationLawsExt::check_justification_order` states that observation is a partial order: reflexive, antisymmetric, and transitive, with every message observing its direct justifications and none justifying itself. `QuorumAdvancePolicy` is the reference advance policy.

An advance policy is a window composed with a `QuorumThreshold`. Admission is derived from the carrier's observations, so every interpreter accepts and rejects the same inputs.

`CommunicationSorts` inherits the validator names from `ValidatorSorts`. Message lookup and justification inherit the same message vocabulary. DAG-specific quorum selections live here; generic quorum formation lives in `ccp-quorum`.

Concrete interpreters live under `crates/`. Portable obligations live with the capabilities they constrain.

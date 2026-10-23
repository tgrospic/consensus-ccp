# ccp-leader

Leader selection, cadence, and products of candidate, horizon, and scheduling policies.

The specification surface is `LeaderSelection`, `LeaderSelectionPolicy`, `CandidateAuthor`, the `Policy` product, and the leader laws. Reference policies (`AllLeaderSlots`, `PeriodicRoundRobin`, `ReferencePolicy`) are concrete scheduling choices that protocol specifications select as parameters.

Candidate authorship inherits candidate and validator sorts. Representation-neutral scheduling policies remain polymorphic over the slots and validators supplied to them.

Concrete interpreters live under `crates/`. Portable obligations live with the capabilities they constrain.

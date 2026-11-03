# ccp-validator

Validator committee, voting weight, committee order, weighted quorum arithmetic, and validator rotation.

The public surface includes `ValidatorSorts`, `ValidatorSet`, `ValidatorWeight`, `CommitteeOrder`, `QuorumThreshold`, `TwoThirdsQuorum`, and their laws.

The safety laws state that two quorums share weight from one half upward and share more than the fault bound from two thirds upward (`quorums_outweigh_faults`), and that a handover without turnover keeps that fault bound while turnover never raises it. `AccountableSafetyLawsExt::check_commit_safety` states accountable safety: two commits of different candidates in one slot, each backed by a quorum, are backed together by validators weighing more than the fault bound.

A quorum is distinct committee support whose weight strictly exceeds a threshold fraction of the total committee weight, or is the whole committee weight. The threshold 1/1 therefore means unanimity, which a genesis committee with one validator meets. Counting validators is the interpretation where every weight is one. This spec depends on no other CCP spec, so validator-management systems can implement it without the evidence layer.

## Validator rotation

A committee stays fixed for an epoch. `CommitteeSchedule` (over `EpochSorts`) tells each epoch's committee in committee order with its weights, a function from validators to weights that is zero outside the committee. `EpochCommittee` reads one epoch as a committee, so every committee and quorum law applies to it.

`CommitteeRotationExt` derives rotation as the difference of two of those functions:

- `rotation(a, b)` splits `committee(b) - committee(a)` into a `Rotation` of validators entering, leaving, and reweighted (`Reweighting`).
- `turnover(a, b)` is the total weight that difference moves.
- `handover_quorums_intersect(threshold, a, b)` holds when every quorum of `a` shares weight in `a` with every quorum of `b`. With threshold `n/d`, turnover `t`, and the weight `T` of `a`, that is `(n + d) * t < (2n - d) * T`, from `HandoverThresholdExt::handover_intersects`. At two thirds it is `5t < T`. A threshold of one half or less tolerates no turnover.

Intersecting quorums are not safety on their own, since the shared validators may be faulty. `HandoverThresholdExt` derives what the overlap outweighs:

- `handover_overlap(T, t)` is the least weight of `a` that a quorum of `a` and a quorum of `b` share: `quorum_size(T) + quorum_size(T - t) - t - T`, or zero.
- `handover_fault_bound(T, t)` is the most faulty weight of `a` the handover tolerates: the overlap still outweighs it, and it stays within `a`'s own `fault_bound`. With no turnover it is `fault_bound(T)` from two thirds upward.

Turnover spends the fault budget. At two thirds, a committee of 24 tolerates 7 faulty weight with no turnover, 4 with a turnover of 3 (an eighth), and none from a turnover of 5. A churn limit therefore buys quorum intersection, not full Byzantine safety across epochs. Full safety comes from two further rules a protocol states: each certificate is weighed only by its own epoch's committee, and the incoming committee continues from the outgoing committee's decided prefix. With both, each epoch needs only its own `fault_bound`.

`ChurnLimit` is the policy that bounds turnover per epoch from the outgoing committee's weight.

| Law | Property |
| --- | --- |
| `check_epoch_committee` | each epoch's committee lists each validator once, in an order that denotes it |
| `check_rotation_identity` | an epoch does not rotate into itself, and turns over nothing |
| `check_rotation_reconstructs` | applying `rotation(a, b)` to `committee(a)` gives exactly `committee(b)`, and an empty rotation turns over nothing |
| `check_rotation_inverse` | rotating back swaps who enters and who leaves and reverses every reweighting |
| `check_turnover_distance` | turnover is a distance: zero to itself, symmetric, never shorter through an epoch in between, at least the change in total weight, and at most both totals |
| `check_handover_overlap` | a quorum of the outgoing committee and a quorum of the incoming committee share at least `handover_overlap` in the outgoing committee, so they outweigh `handover_fault_bound` faulty weight, and share weight whenever the quorums intersect |
| `check_churn` | a rotation stays within the churn limit, a limit the threshold tolerates keeps quorums intersecting, and the rotation tolerates at least the faults the limit leaves |
| `check_churn_limit` | a churn limit never exceeds the committee, keeps quorums intersecting under the threshold, and never shrinks as the committee grows |

## Epoch binding

`SlotEpoch` places each slot in an epoch. `EpochBindingExt` derives `slot_committee(slot)`, the committee of the slot's epoch, and `slot_quorum(threshold, slot, signers)`, a quorum of that committee. A certificate weighed only by the committee of its own slot's epoch keeps safety a property of each epoch: two conflicting certificates in one slot are weighed by the same committee, so their shared backers outweigh its `fault_bound` however much rotated before or after it. `EpochBindingLawsExt` states it:

| Law | Property |
| --- | --- |
| `check_slot_epochs` | a later slot never falls in an earlier epoch |
| `check_committee_binding` | the committee a protocol weighs evidence of a slot with is the committee of the slot's epoch, with the same weights |
| `check_epoch_commit_safety` | two quorum-backed commits of different candidates in one slot share backers weighing more than the fault bound of the slot's epoch committee |

The other rule, that decisions after a handover extend the prefix closed before it, is a law on collapse and lives in `ccp-core` as `HandoverLawsExt::check_handover_continuity`.

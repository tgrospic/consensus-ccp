# ccp-stake

States what proof of stake means in a form general enough for its variants. The core is one function: a validator has a weight in an epoch, recorded when a snapshot was taken. Everything that differs between chains is a separate algebra around that core: where weight comes from, which snapshot governs an epoch, who is excluded, how rewards are emitted and shared, and whether proven offenses forfeit stake.

The crate is a specification. It depends on [`alux-ext`](https://docs.rs/alux-ext) and on the validator specification [`ccp-validator`](https://github.com/tgrospic/consensus-ccp), and names no token, consensus protocol, or runtime. Consensus enters only as primitive observations: [`PerformanceAlg`] says how well a validator did its duties, and the offense a [`ForfeitAlg`] prices is evidence consensus proved. Leader selection and finality rules belong to a consensus specification that consumes epoch weights.

## Core

| Algebra | Meaning |
| --- | --- |
| [`EpochAlg`] | the current epoch |
| [`WeightSourceAlg`] | how much stake stands behind each validator right now |
| [`SnapshotAlg`] | the weights recorded when each epoch closed, and the latest one before an epoch |
| [`SnapshotLagAlg`] | which snapshot governs weight in an epoch |
| [`ExclusionAlg`] | validators kept out of weight, and until when |

[`StakeWeightExt`] derives a snapshot's target (the source's weights of validators not excluded), the weights already admitted, taking a snapshot, excluding a validator, and reading weights and their total in an epoch. [`StakeQuorumExt`] reads epoch weights as a `ccp-validator` committee schedule, [`StakeSchedule`], and an epoch's weights as its committee, whose validators weigh their stake. A chain with quorum finality answers `ccp_validator::QuorumThreshold`, and its quorum is that committee's weighted quorum.

## Validator rotation

An epoch's weights are a function from validators to amounts, zero outside the set:

```text
weights(e) = snapshot(governing(e)), less validators excluded when it was taken
```

Validator rotation is nothing more than the difference between two of those functions. What rotation means belongs to `ccp-validator`: [`StakeSchedule`] is a `CommitteeSchedule`, so `CommitteeRotationExt` derives `rotation(a, b)`, `turnover(a, b)`, `handover_quorums_intersect`, and the fault bound a handover leaves, and `RotationLawsExt` states their laws. Bonding, unbonding, delegating, spending delegated holdings, forfeiting, and exclusion all rotate validators, and only through the weight source and the next snapshot.

[`StakeSettlementLawsExt`] states what stake adds to rotation:

| Law | Property |
| --- | --- |
| `check_snapshot_precedes` | the snapshot governing an epoch after genesis is taken before the epoch begins |
| `check_weights_settled` | nothing done during an epoch changes that epoch's weights; rotation happens only at epoch boundaries |

Settlement is what consensus relies on: a protocol hands over between epochs knowing both committees in advance, and `check_handover_overlap` states how much quorums on either side share.

Rotation spends the fault budget, so a churn limit buys quorum intersection, not full safety across epochs. Safety comes from two rules a protocol states: each certificate is weighed only by its own epoch's committee (`EpochBindingLawsExt` in `ccp-validator`, see its "Epoch binding" section), and decisions after a handover extend the prefix closed before it (`HandoverLawsExt::check_handover_continuity` in `ccp-core`). With both, each epoch needs only its own fault bound, however much stake rotates.

## Churn queue

A chain that limits rotation answers `ccp_validator::ChurnLimit` and takes its snapshots through [`StakeChurnExt`] instead of taking the whole target. The queue between the weight source and the snapshot is not stored: it is the difference between the weights already admitted and the target, and `stake_churn_pending` is its size. Each snapshot moves the admitted weights toward the target by at most the churn limit of the admitted total: admitted validators first, in their order, then entering validators in the source's order, with the last change possibly partial. The first snapshot admits its whole target, since there is no committee to hand over from. Variants without a limit take the whole target with `stake_snapshot_take`.

| Law | Property |
| --- | --- |
| `check_snapshot_admit` | each weight moves toward its target and never past it, the turnover admitted is the queued turnover capped by the limit, and exactly that much leaves the queue |

Consecutive snapshots then satisfy `ccp-validator`'s `check_churn`, so a limit the threshold tolerates keeps quorums intersecting across every handover. It does not keep the full fault bound: turnover spends it, as `handover_fault_bound` states.

## Rewards

| Algebra | Meaning |
| --- | --- |
| [`PerformanceAlg`] | how well a validator did its duties, as consensus observed |
| [`EmissionAlg`] | the epoch's reward pot |
| [`RewardShareAlg`] | what a validator earned from the pot |
| [`RewardPayoutAlg`] | paying rewards with value |

[`StakeRewardsExt`] settles an epoch: paid and unpaid always sum to the pot. Withholding rewards for weak performance is a share policy, not a separate penalty, and it never touches principal.

## Weight sources

| Source | Algebras | Extension |
| --- | --- | --- |
| Escrow: tokens are locked, leave after a delay, and may be forfeited | [`BondsAlg`], [`UnbondingAlg`], [`UnbondingDelayAlg`], [`CustodyAlg`], [`ForfeitAlg`] | [`StakeEscrowExt`], [`StakeForfeitExt`] |
| Delegation: holders point their holdings at a validator and nothing moves | [`DelegationAlg`], [`HoldingsAlg`] | [`StakeDelegationExt`] |

An interpretation answers [`WeightSourceAlg`] by delegating to `escrow_weights` or `delegated_weights`.

Escrowed stake must still answer for every epoch it weighed. [`EvidenceWindowAlg`] says until when consensus acts on evidence of an epoch, and [`StakeAccountabilityLawsExt`] relates it to the snapshot lag and the unbonding delay:

| Law | Property |
| --- | --- |
| `check_unbonding_answers` | evidence of an epoch is acted on at least during it, and stake that weighed in an epoch matures no earlier than evidence of that epoch expires |

A delay shorter than the lag plus the evidence window lets a validator withdraw before its offense is proven, the long-range attack.

## Variants

A variant is a choice of primitive interpretations, not a different specification:

| Variant | Weight source | Snapshot lag | Rewards | Misbehavior costs |
| --- | --- | --- | --- | --- |
| Cosmos-like | escrow | one epoch | inflation, by weight | forfeit and exclusion (jailing) |
| Ethereum-like | escrow, through a churn queue | one epoch | per duty performed | forfeit, exclusion (ejection), and slow forfeit for inactivity |
| Cardano-like | delegation | two epochs | reserve, by weight and performance | withheld rewards only |

TODO: record recent forfeits as a primitive observation, so a forfeit policy can grow with how much other stake was forfeited in the same window, as Ethereum's correlation penalty does.

## Consensus

TODO: run the Mysticeti and Cordial scenarios with a committee taken from epoch weights instead of unit weights; the runtime carrier needs a weighted genesis committee for this.

The handover laws are stated: `ccp-validator`'s `EpochBindingLawsExt` and `ccp-core`'s `HandoverLawsExt`. A vote protocol whose slots span two epochs witnesses them in the conformance tests.

TODO: let a runtime communication history span epochs, one committee per epoch from a `StakeSchedule`, so the Mysticeti and Cordial scenarios run the handover laws across a real committee change.

TODO: derive [`PerformanceAlg`] from CCP evidence: the messages a validator authored against the rounds or leader slots assigned to it.

TODO: prove the offense a [`ForfeitAlg`] prices from CCP equivocation evidence.

## Laws

Each extension has a law extension that every interpretation runs over the values it chooses: [`StakeWeightLawsExt`], [`StakeSettlementLawsExt`], [`StakeQuorumLawsExt`], [`StakeRewardsLawsExt`], [`StakeEscrowLawsExt`], [`StakeForfeitLawsExt`], [`StakeDelegationLawsExt`], [`StakeChurnLawsExt`], and [`StakeAccountabilityLawsExt`]. A snapshot records exactly the source's weights of validators not excluded; rotation is settled and is the difference of epoch weights; an epoch's committee satisfies the `ccp-validator` laws, and its stake quorum is the committee's weighted quorum; paid and unpaid rewards sum to the pot and leave weights unchanged; escrow moves exactly the amounts asked; forfeiting takes exactly what it reports and only from the offender; delegating moves no value; the churn queue admits exactly what the limit allows; unbonding outlasts the evidence of every epoch the stake weighed. [`ccp-stake-memory`](https://docs.rs/ccp-stake-memory) is the reference interpretation that runs them.

A consensus specification consumes stake through its bounds alone. Accepting a block certificate is an extension over epoch weights and quorum:

```rust
use alux_ext::ext;
use ccp_stake::{
    EpochAlg, ExclusionAlg, SnapshotAlg, SnapshotLagAlg, StakeQuorumExt, WeightSourceAlg,
};
use ccp_validator::QuorumThreshold;
use std::{hash::Hash, ops::Add};

/// Derives certificate acceptance from stake-weighted quorum.
#[ext(name = CertificateExt)]
pub impl<This, Validator, Amount, Epoch> This
where
    This: EpochAlg<Epoch = Epoch>
        + WeightSourceAlg<Validator = Validator, Amount = Amount>
        + SnapshotAlg<Validator = Validator, Amount = Amount, Epoch = Epoch>
        + SnapshotLagAlg<Epoch = Epoch>
        + ExclusionAlg<Validator = Validator, Epoch = Epoch>
        + QuorumThreshold,
    Validator: Clone + Eq + Hash,
    Amount: Copy + Ord + Default + Add<Output = Amount> + Into<u128>,
    Epoch: Clone + Ord,
{
    /// Accepts a block when its signers reach quorum in its epoch.
    fn certificate_accepts<Block>(&self, block: Block, epoch: &Epoch, signers: &[Validator]) -> Option<Block> {
        self.stake_quorum(epoch, signers).then_some(block)
    }
}
```

# ccp-stake

States what proof of stake means in a form general enough for its variants. The core is one function: a validator has a weight in an epoch, recorded when a snapshot was taken. Everything that differs between chains is a separate algebra around that core: where weight comes from, which snapshot governs an epoch, who is excluded, how rewards are emitted and shared, and whether proven offenses forfeit stake.

The crate is a specification. It depends on [`alux-ext`](https://docs.rs/alux-ext) and on the validator specification [`ccp-validator`](https://github.com/tgrospic/consensus-ccp), and names no token, consensus protocol, or runtime. Consensus enters only as primitive observations: [`PerformanceAlg`] says how well a validator did its duties, and the offense a [`ForfeitAlg`] prices is evidence consensus proved. Leader selection and finality rules belong to a consensus specification that consumes epoch weights.

## Core

| Algebra | Meaning |
| --- | --- |
| [`EpochAlg`] | the current epoch |
| [`WeightSourceAlg`] | how much stake stands behind each validator right now |
| [`SnapshotAlg`] | the weights recorded when each epoch closed |
| [`SnapshotLagAlg`] | which snapshot governs weight in an epoch |
| [`ExclusionAlg`] | validators kept out of weight, and until when |

[`StakeWeightExt`] derives taking a snapshot, excluding a validator, and reading weights and their total in an epoch. [`StakeQuorumExt`] reads an epoch's weights as a `ccp-validator` committee, [`EpochCommittee`], whose validators weigh their stake. A chain with quorum finality answers `ccp_validator::QuorumThreshold`, and its quorum is that committee's weighted quorum.

## Validator rotation

An epoch's weights are a function from validators to amounts, zero outside the set:

```text
weights(e) = snapshot(governing(e)), less validators excluded when it was taken
```

Validator rotation is nothing more than the difference between two of those functions. [`StakeRotationExt`] derives it: `stake_rotation(a, b)` splits `weights(b) - weights(a)` into a [`Rotation`] of validators entering, leaving, and reweighted, and `stake_turnover(a, b)` sums how much weight that difference moves. Bonding, unbonding, delegating, spending delegated holdings, forfeiting, and exclusion all rotate validators, and only through the weight source and the next snapshot.

[`StakeRotationLawsExt`] states what the specification cares about:

| Law | Property |
| --- | --- |
| `check_snapshot_precedes` | the snapshot governing an epoch after genesis is taken before the epoch begins |
| `check_weights_settled` | nothing done during an epoch changes that epoch's weights; rotation happens only at epoch boundaries |
| `check_rotation_identity` | an epoch does not rotate into itself |
| `check_rotation_reconstructs` | applying `stake_rotation(a, b)` to `weights(a)` gives exactly `weights(b)` |
| `check_rotation_inverse` | rotating back swaps who enters and who leaves and reverses every reweighting |
| `check_turnover_distance` | turnover is a distance between epochs: zero to itself, symmetric, and never shorter through an epoch in between |

Settlement is what consensus relies on. [`StakeQuorumLawsExt`] runs the `ccp-validator` laws on an epoch's committee: two quorums of the same epoch share a validator when the threshold is at least one half. Quorums of different epochs need not, so a consensus protocol hands over between epochs knowing both sides' weights in advance, which settlement guarantees. How fast weights may rotate is a policy: turnover per epoch is the quantity a churn limit bounds.

TODO: a law bounding `stake_turnover` between consecutive epochs by a churn limit, once the specification has the queue that enforces it.

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

## Variants

A variant is a choice of primitive interpretations, not a different specification:

| Variant | Weight source | Snapshot lag | Rewards | Misbehavior costs |
| --- | --- | --- | --- | --- |
| Cosmos-like | escrow | one epoch | inflation, by weight | forfeit and exclusion (jailing) |
| Ethereum-like | escrow | one epoch | per duty performed | forfeit, exclusion (ejection), and slow forfeit for inactivity |
| Cardano-like | delegation | two epochs | reserve, by weight and performance | withheld rewards only |

TODO: add a queue between the weight source and the snapshot that limits how much weight enters and leaves per epoch, so an interpretation can state Ethereum's activation and exit churn limit. Variants without a limit answer with an unbounded queue.

TODO: record recent forfeits as a primitive observation, so a forfeit policy can grow with how much other stake was forfeited in the same window, as Ethereum's correlation penalty does.

## Consensus

TODO: run the Mysticeti and Cordial scenarios with a committee taken from epoch weights instead of unit weights; the runtime carrier needs a weighted genesis committee for this.

TODO: let a protocol history span epochs, one committee per epoch with weights settled before the epoch begins, and state how a protocol hands over between epochs.

TODO: derive [`PerformanceAlg`] from CCP evidence: the messages a validator authored against the rounds or leader slots assigned to it.

TODO: prove the offense a [`ForfeitAlg`] prices from CCP equivocation evidence.

## Laws

Each extension has a law extension that every interpretation runs over the values it chooses: [`StakeWeightLawsExt`], [`StakeRotationLawsExt`], [`StakeQuorumLawsExt`], [`StakeRewardsLawsExt`], [`StakeEscrowLawsExt`], [`StakeForfeitLawsExt`], and [`StakeDelegationLawsExt`]. A snapshot records exactly the source's weights of validators not excluded; rotation is settled and is the difference of epoch weights; an epoch's committee satisfies the `ccp-validator` laws, and its stake quorum is the committee's weighted quorum; paid and unpaid rewards sum to the pot and leave weights unchanged; escrow moves exactly the amounts asked; forfeiting takes exactly what it reports and only from the offender; delegating moves no value. [`ccp-stake-memory`](https://docs.rs/ccp-stake-memory) is the reference interpretation that runs them.

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

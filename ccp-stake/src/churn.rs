use crate::{EpochAlg, ExclusionAlg, SnapshotAlg, SnapshotLagAlg, StakeWeightExt, WeightSourceAlg};
use alux_ext::ext;
use ccp_validator::ChurnLimit;
use std::ops::{Add, Sub};

/// Derives the churn queue between the weight source and the snapshot.
///
/// The queue is not stored: it is the difference between the weights already admitted and the
/// snapshot's target. Each snapshot moves the admitted weights toward the target by at most the
/// `ccp_validator` churn limit of the admitted total, so consecutive snapshots never rotate more than
/// the limit allows. Changes are admitted for the admitted validators first, in their order, then for
/// entering validators in the source's order; the last change admitted may be partial, and the rest
/// stays queued. With nothing admitted yet there is no committee to hand over from, so the first
/// snapshot admits its whole target.
#[ext(name = StakeChurnExt)]
pub impl<This, Validator, Amount, Epoch> This
where
    This: EpochAlg<Epoch = Epoch>
        + WeightSourceAlg<Validator = Validator, Amount = Amount>
        + SnapshotAlg<Validator = Validator, Amount = Amount, Epoch = Epoch>
        + SnapshotLagAlg<Epoch = Epoch>
        + ExclusionAlg<Validator = Validator, Epoch = Epoch>
        + ChurnLimit,
    Validator: Clone + PartialEq,
    Amount: Copy + Ord + Default + Add<Output = Amount> + Sub<Output = Amount> + Into<u128> + TryFrom<u128>,
    Epoch: Clone + Ord,
{
    /// Returns how much turnover is still queued: the distance from the admitted weights to the
    /// snapshot's target.
    fn stake_churn_pending(&self) -> u128 {
        weight_distance(&self.stake_admitted(), &self.stake_snapshot_target())
    }

    /// Records the current epoch's snapshot as the admitted weights moved toward the target by at
    /// most the churn limit.
    fn stake_snapshot_admit(&mut self) {
        let (admitted, target) = (self.stake_admitted(), self.stake_snapshot_target());
        let total = admitted.iter().map(|(_, weight)| (*weight).into()).sum::<u128>();
        let weights = match Amount::try_from(self.churn_limit(total)) {
            Ok(budget) if total > 0 => churn_toward(admitted, &target, budget),
            _ => target,
        };
        self.snapshot_record(self.epoch(), weights);
    }
}

/// Returns the turnover between two weight lists: the sum of each validator's weight change.
pub(crate) fn weight_distance<Validator, Amount>(from: &[(Validator, Amount)], to: &[(Validator, Amount)]) -> u128
where
    Validator: PartialEq,
    Amount: Copy + Ord + Default + Sub<Output = Amount> + Into<u128>,
{
    let leaving = from.iter().filter(|(validator, _)| !to.iter().any(|(kept, _)| kept == validator));
    to.iter()
        .chain(leaving)
        .map(|(validator, _)| {
            let (before, after) = (weight_in(from, validator), weight_in(to, validator));
            (before.max(after) - before.min(after)).into()
        })
        .sum()
}

/// Returns the validator's weight in the list; zero when it is absent.
pub(crate) fn weight_in<Validator, Amount>(weights: &[(Validator, Amount)], validator: &Validator) -> Amount
where
    Validator: PartialEq,
    Amount: Copy + Default,
{
    weights.iter().find(|(kept, _)| kept == validator).map_or_else(Amount::default, |(_, weight)| *weight)
}

/// Moves each weight toward the target while the budget lasts.
fn churn_toward<Validator, Amount>(
    from: Vec<(Validator, Amount)>,
    to: &[(Validator, Amount)],
    mut budget: Amount,
) -> Vec<(Validator, Amount)>
where
    Validator: Clone + PartialEq,
    Amount: Copy + Ord + Default + Add<Output = Amount> + Sub<Output = Amount>,
{
    let entering: Vec<_> = to
        .iter()
        .filter(|(validator, _)| !from.iter().any(|(kept, _)| kept == validator))
        .map(|(validator, _)| (validator.clone(), Amount::default()))
        .collect();
    from.into_iter()
        .chain(entering)
        .map(|(validator, weight)| {
            let goal = weight_in(to, &validator);
            let step = (goal.max(weight) - goal.min(weight)).min(budget);
            budget = budget - step;
            let moved = if goal > weight { weight + step } else { weight - step };
            (validator, moved)
        })
        .filter(|(_, weight)| *weight > Amount::default())
        .collect()
}

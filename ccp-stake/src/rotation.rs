use crate::{
    EpochAlg, ExclusionAlg, Reweighting, Rotation, SnapshotAlg, SnapshotLagAlg, StakeWeightExt, WeightSourceAlg,
};
use alux_ext::ext;
use std::ops::{Add, Sub};

/// Derives validator rotation: the difference between two epochs' weight distributions.
///
/// An epoch's weights are a function from validators to amounts, zero outside the set. Rotation from
/// one epoch to another is the pointwise difference of those functions, split by whether a validator
/// enters, leaves, or stays with another weight. Turnover is the total weight that difference moves:
/// the distance between the two distributions.
#[ext(name = StakeRotationExt)]
pub impl<This, Validator, Amount, Epoch> This
where
    This: EpochAlg<Epoch = Epoch>
        + WeightSourceAlg<Validator = Validator, Amount = Amount>
        + SnapshotAlg<Validator = Validator, Amount = Amount, Epoch = Epoch>
        + SnapshotLagAlg<Epoch = Epoch>
        + ExclusionAlg<Validator = Validator, Epoch = Epoch>,
    Validator: Clone + PartialEq,
    Amount: Copy + Ord + Default + Add<Output = Amount> + Sub<Output = Amount>,
    Epoch: Clone + Ord,
{
    /// Returns how the weight distribution changes from one epoch to another.
    fn stake_rotation(&self, from: &Epoch, to: &Epoch) -> Rotation<Validator, Amount> {
        let before = self.stake_weights(from);
        let after = self.stake_weights(to);
        let weight_in = |weights: &[(Validator, Amount)], validator: &Validator| {
            weights.iter().find(|(kept, _)| kept == validator).map(|(_, weight)| *weight)
        };
        let entering = after.iter().filter(|(validator, _)| weight_in(&before, validator).is_none()).cloned().collect();
        let leaving = before.iter().filter(|(validator, _)| weight_in(&after, validator).is_none()).cloned().collect();
        let reweighted = before
            .iter()
            .filter_map(|(validator, from)| {
                let to = weight_in(&after, validator)?;
                (to != *from).then(|| Reweighting { validator: validator.clone(), from: *from, to })
            })
            .collect();
        Rotation { entering, leaving, reweighted }
    }

    /// Returns the total weight rotation moves from one epoch to another: what enters, what leaves,
    /// and how far each staying weight changes.
    fn stake_turnover(&self, from: &Epoch, to: &Epoch) -> Amount {
        let rotation = self.stake_rotation(from, to);
        let moved = |weights: &[(Validator, Amount)]| {
            weights.iter().map(|(_, weight)| *weight).fold(Amount::default(), Add::add)
        };
        rotation
            .reweighted
            .iter()
            .map(|change| change.from.max(change.to) - change.from.min(change.to))
            .fold(moved(&rotation.entering) + moved(&rotation.leaving), Add::add)
    }
}

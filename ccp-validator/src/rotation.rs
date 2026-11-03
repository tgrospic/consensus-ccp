//! Defines validator rotation: how the committee changes from one epoch to another.
use crate::{
    CommitteeOrder, EpochSorts, QuorumThreshold, QuorumThresholdExt, ValidatorSet, ValidatorSorts, ValidatorWeight,
};
use alux_ext::ext;
use std::ops::{Add, Sub};

/// Tells the weighted committee of each epoch.
///
/// An epoch's committee is a function from validators to weights, zero outside the committee.
pub trait CommitteeSchedule: ValidatorSorts + EpochSorts {
    /// Returns every validator of `epoch` once, in committee order, with its weight.
    fn committee_at(&self, epoch: &Self::Epoch) -> Vec<(Self::Validator, Self::Weight)>;
}

/// Bounds how much weight may rotate from one epoch to the next.
pub trait ChurnLimit {
    /// Returns the most turnover allowed into the next epoch when the current epoch weighs `total`.
    fn churn_limit(&self, total: u128) -> u128;
}

/// Holds how the committee changed from one epoch to another: who entered, who left, and whose
/// weight moved.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Rotation<Validator, Weight> {
    /// Holds every validator in the later committee and not the earlier, with its weight.
    pub entering: Vec<(Validator, Weight)>,
    /// Holds every validator in the earlier committee and not the later, with its weight.
    pub leaving: Vec<(Validator, Weight)>,
    /// Holds every validator in both committees whose weight differs.
    pub reweighted: Vec<Reweighting<Validator, Weight>>,
}

/// Holds one validator's weight in two epochs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Reweighting<Validator, Weight> {
    /// Identifies the validator.
    pub validator: Validator,
    /// Holds its weight in the earlier epoch.
    pub from: Weight,
    /// Holds its weight in the later epoch.
    pub to: Weight,
}

impl<Schedule> ValidatorSorts for &Schedule
where
    Schedule: ValidatorSorts + ?Sized,
{
    type Validator = Schedule::Validator;
    type Weight = Schedule::Weight;
}

impl<Schedule> EpochSorts for &Schedule
where
    Schedule: EpochSorts + ?Sized,
{
    type Epoch = Schedule::Epoch;
}

impl<Schedule> CommitteeSchedule for &Schedule
where
    Schedule: CommitteeSchedule + ?Sized,
{
    fn committee_at(&self, epoch: &Self::Epoch) -> Vec<(Self::Validator, Self::Weight)> {
        (**self).committee_at(epoch)
    }
}

/// Reads one epoch of a schedule as a committee, so every committee law applies to it.
#[derive(Debug, Clone, Copy)]
pub struct EpochCommittee<Schedule, Epoch> {
    /// The schedule the committee is read from.
    pub schedule: Schedule,
    /// The epoch whose committee this is.
    pub epoch: Epoch,
}

impl<Schedule, Epoch> ValidatorSorts for EpochCommittee<Schedule, Epoch>
where
    Schedule: ValidatorSorts,
{
    type Validator = Schedule::Validator;
    type Weight = Schedule::Weight;
}

impl<Schedule, V, W, Epoch> ValidatorSet for EpochCommittee<Schedule, Epoch>
where
    Schedule: CommitteeSchedule<Validator = V, Weight = W, Epoch = Epoch>,
{
    fn validators(&self) -> Vec<V> {
        self.committee_order()
    }
}

impl<Schedule, V, W, Epoch> CommitteeOrder for EpochCommittee<Schedule, Epoch>
where
    Schedule: CommitteeSchedule<Validator = V, Weight = W, Epoch = Epoch>,
{
    fn committee_order(&self) -> Vec<V> {
        self.schedule.committee_at(&self.epoch).into_iter().map(|(validator, _)| validator).collect()
    }
}

impl<Schedule, V, W, Epoch> ValidatorWeight for EpochCommittee<Schedule, Epoch>
where
    Schedule: CommitteeSchedule<Validator = V, Weight = W, Epoch = Epoch>,
    V: PartialEq,
    W: Default,
{
    fn weight(&self, validator: &V) -> W {
        self.schedule
            .committee_at(&self.epoch)
            .into_iter()
            .find(|(member, _)| member == validator)
            .map_or_else(W::default, |(_, weight)| weight)
    }
}

/// Derives rotation, turnover, and whether quorums intersect across a handover from a committee
/// schedule.
///
/// Rotation from one epoch to another is the pointwise difference of their weight functions, split
/// by whether a validator enters, leaves, or stays with another weight. Turnover is the total
/// weight that difference moves: the distance between the two committees.
#[ext(name = CommitteeRotationExt)]
pub impl<This, V, W, E> This
where
    This: CommitteeSchedule<Validator = V, Weight = W, Epoch = E>,
    V: Clone + PartialEq,
    W: Copy + Ord + Default + Add<Output = W> + Sub<Output = W>,
    E: Clone,
{
    /// Returns the committee of `epoch`.
    fn epoch_committee(&self, epoch: &E) -> EpochCommittee<&This, E> {
        EpochCommittee { schedule: self, epoch: epoch.clone() }
    }

    /// Returns the sum of every weight in the committee of `epoch`.
    fn epoch_total(&self, epoch: &E) -> W {
        self.committee_at(epoch).into_iter().map(|(_, weight)| weight).fold(W::default(), Add::add)
    }

    /// Returns how the committee changes from one epoch to another.
    fn rotation(&self, from: &E, to: &E) -> Rotation<V, W> {
        let before = self.committee_at(from);
        let after = self.committee_at(to);
        let weight_in = |committee: &[(V, W)], validator: &V| {
            committee.iter().find(|(member, _)| member == validator).map(|(_, weight)| *weight)
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
    fn turnover(&self, from: &E, to: &E) -> W {
        let rotation = self.rotation(from, to);
        let moved = |committee: &[(V, W)]| committee.iter().map(|(_, weight)| *weight).fold(W::default(), Add::add);
        rotation
            .reweighted
            .iter()
            .map(|change| change.from.max(change.to) - change.from.min(change.to))
            .fold(moved(&rotation.entering) + moved(&rotation.leaving), Add::add)
    }

    /// Reports whether every quorum of `from` shares weight with every quorum of `to` under
    /// `threshold`.
    ///
    /// With threshold `n/d` and turnover `t` out of the weight `T` of `from`, two such quorums share
    /// weight in `from` when `(n + d) * t < (2n - d) * T`. At two thirds this is `5t < T`. Sharing
    /// weight is not safety on its own: the shared validators may be faulty, and how much faulty
    /// weight the overlap still outweighs is [`HandoverThresholdExt::handover_fault_bound`].
    fn handover_quorums_intersect<Threshold>(&self, threshold: &Threshold, from: &E, to: &E) -> bool
    where
        Threshold: QuorumThreshold,
        W: Into<u128>,
    {
        threshold.handover_intersects(self.epoch_total(from).into(), self.turnover(from, to).into())
    }
}

/// Derives what a quorum threshold guarantees across a handover in which `turnover` out of the
/// outgoing committee's `total` weight rotates.
#[ext(name = HandoverThresholdExt)]
pub impl<This> This
where
    This: QuorumThreshold,
{
    /// Reports whether quorums of two committees share weight in the first when `turnover` out of
    /// its `total` weight rotates between them: `(n + d) * turnover < (2n - d) * total`.
    fn handover_intersects(&self, total: u128, turnover: u128) -> bool {
        let numerator = u128::from(self.threshold_numerator());
        let denominator = u128::from(self.threshold_denominator());
        self.quorums_intersect()
            && (numerator + denominator).saturating_mul(turnover)
                < (2 * numerator).saturating_sub(denominator).saturating_mul(total)
    }

    /// Returns the least weight in the outgoing committee that a quorum of it and a quorum of the
    /// incoming committee always share.
    ///
    /// The outgoing quorum weighs at least `quorum_size(total)`. The incoming committee weighs at
    /// least `total - turnover`, so its quorum weighs at least `quorum_size(total - turnover)` there
    /// and loses at most `turnover` read in the outgoing committee. Two sets within `total` overlap
    /// by at least the excess of their weights over it.
    fn handover_overlap(&self, total: u128, turnover: u128) -> u128 {
        let incoming = self.quorum_size(total.saturating_sub(turnover)).saturating_sub(turnover);
        self.quorum_size(total).saturating_add(incoming).saturating_sub(total)
    }

    /// Returns the most faulty weight of the outgoing committee that the handover tolerates: the
    /// overlap of any two quorums across it still outweighs that faulty weight, and the rest of the
    /// committee still reaches a quorum.
    ///
    /// With no turnover it is the committee's own `fault_bound` from two thirds upward. Turnover
    /// spends it: at two thirds it falls by about `5/3` of the turnover and is gone near a fifth.
    fn handover_fault_bound(&self, total: u128, turnover: u128) -> u128 {
        self.fault_bound(total).min(self.handover_overlap(total, turnover).saturating_sub(1))
    }
}

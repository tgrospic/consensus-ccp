//! Binds evidence to the epoch whose committee weighs it.
use crate::{
    CommitteeRotationExt, CommitteeSchedule, EpochCommittee, EpochSorts, QuorumThreshold, WeightedCommitteeExt,
};
use alux_ext::ext;
use std::{
    hash::Hash,
    ops::{Add, Sub},
};

/// Places each slot in an epoch: evidence produced at a slot is weighed by that epoch's committee.
pub trait SlotEpoch: EpochSorts {
    /// Names a position at which evidence is produced.
    type Slot;

    /// Returns the epoch of `slot`.
    fn epoch_of(&self, slot: &Self::Slot) -> Self::Epoch;
}

/// Derives the committee and the quorum that weigh evidence produced at a slot.
///
/// Weighing a certificate only by the committee of its own epoch makes safety a property of each
/// epoch: two conflicting certificates in one slot are weighed by the same committee, so they are
/// accountable to more than that committee's fault bound whatever rotated before or after it.
#[ext(name = EpochBindingExt)]
pub impl<This, V, W, E, S> This
where
    This: CommitteeSchedule<Validator = V, Weight = W, Epoch = E> + SlotEpoch<Slot = S>,
    V: Clone + PartialEq,
    W: Copy + Ord + Default + Add<Output = W> + Sub<Output = W>,
    E: Clone,
{
    /// Returns the committee that weighs evidence produced at `slot`.
    fn slot_committee(&self, slot: &S) -> EpochCommittee<&This, E> {
        self.epoch_committee(&self.epoch_of(slot))
    }

    /// Reports whether `signers` carry a quorum of the committee of `slot`'s epoch under
    /// `threshold`; signers outside that committee count nothing.
    fn slot_quorum<Threshold, I>(&self, threshold: &Threshold, slot: &S, signers: I) -> bool
    where
        Threshold: QuorumThreshold,
        I: IntoIterator<Item = V>,
        V: Eq + Hash,
        W: Into<u128>,
    {
        self.slot_committee(slot).weighted_quorum(threshold, signers)
    }
}

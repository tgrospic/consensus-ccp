use crate::{EpochAlg, ExclusionAlg, SnapshotAlg, SnapshotLagAlg, StakeWeightExt, WeightSourceAlg};
use alux_ext::ext;
use ccp_validator::{
    CommitteeSchedule, EpochCommittee, EpochSorts, QuorumThreshold, ValidatorSorts, WeightedCommitteeExt,
};
use std::{hash::Hash, ops::Add};

/// Reads epoch weights as a [`ccp_validator`] committee schedule: the committee of an epoch is every
/// validator with weight in it, most weighty first, and each weighs its stake.
///
/// Validator rotation, turnover, and quorum overlap across a handover are the schedule's, derived by
/// [`CommitteeRotationExt`](ccp_validator::CommitteeRotationExt).
#[derive(Debug, Clone, Copy)]
pub struct StakeSchedule<'a, Stake: ?Sized> {
    /// The stake the schedule is read from.
    pub stake: &'a Stake,
}

impl<Stake, Validator, Amount> ValidatorSorts for StakeSchedule<'_, Stake>
where
    Stake: WeightSourceAlg<Validator = Validator, Amount = Amount> + ?Sized,
{
    type Validator = Validator;
    type Weight = Amount;
}

impl<Stake, Epoch> EpochSorts for StakeSchedule<'_, Stake>
where
    Stake: EpochAlg<Epoch = Epoch> + ?Sized,
{
    type Epoch = Epoch;
}

impl<Stake, Validator, Amount, Epoch> CommitteeSchedule for StakeSchedule<'_, Stake>
where
    Stake: EpochAlg<Epoch = Epoch>
        + WeightSourceAlg<Validator = Validator, Amount = Amount>
        + SnapshotAlg<Validator = Validator, Amount = Amount, Epoch = Epoch>
        + SnapshotLagAlg<Epoch = Epoch>
        + ExclusionAlg<Validator = Validator, Epoch = Epoch>,
    Validator: PartialEq,
    Amount: Copy + Ord + Default + Add<Output = Amount>,
    Epoch: Clone + Ord,
{
    fn committee_at(&self, epoch: &Epoch) -> Vec<(Validator, Amount)> {
        self.stake.stake_weights(epoch)
    }
}

/// Derives the committee schedule, an epoch's committee, and its quorum from epoch weights and a
/// threshold.
#[ext(name = StakeQuorumExt)]
pub impl<This, Validator, Amount, Epoch> This
where
    This: EpochAlg<Epoch = Epoch>
        + WeightSourceAlg<Validator = Validator, Amount = Amount>
        + SnapshotAlg<Validator = Validator, Amount = Amount, Epoch = Epoch>
        + SnapshotLagAlg<Epoch = Epoch>
        + ExclusionAlg<Validator = Validator, Epoch = Epoch>,
    Validator: Clone + Eq + Hash,
    Amount: Copy + Ord + Default + Add<Output = Amount> + Into<u128>,
    Epoch: Clone + Ord,
{
    /// Returns the committee schedule the epoch weights denote.
    fn stake_schedule(&self) -> StakeSchedule<'_, Self> {
        StakeSchedule { stake: self }
    }

    /// Returns the committee of `epoch`, weighted by stake.
    fn stake_committee(&self, epoch: &Epoch) -> EpochCommittee<StakeSchedule<'_, Self>, Epoch> {
        EpochCommittee { schedule: self.stake_schedule(), epoch: epoch.clone() }
    }

    /// Says whether the voters' combined weight in the epoch decides; a repeated voter counts once.
    fn stake_quorum(&self, epoch: &Epoch, voters: &[Validator]) -> bool
    where
        This: QuorumThreshold,
    {
        self.stake_committee(epoch).weighted_quorum(self, voters.iter().cloned())
    }
}

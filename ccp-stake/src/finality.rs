use crate::{EpochAlg, ExclusionAlg, SnapshotAlg, SnapshotLagAlg, StakeWeightExt, WeightSourceAlg};
use alux_ext::ext;
use ccp_validator::{QuorumThreshold, ValidatorSet, ValidatorSorts, ValidatorWeight, WeightedCommitteeExt};
use std::{hash::Hash, ops::Add};

/// Reads the validators of one epoch as a [`ccp_validator`] committee weighted by their stake.
///
/// The committee is every validator with weight in the epoch, and each weighs its stake. A chain
/// with quorum finality also answers [`QuorumThreshold`], and its quorum is the committee's.
#[derive(Debug, Clone, Copy)]
pub struct EpochCommittee<'a, Stake: ?Sized, Epoch> {
    /// The stake the committee is read from.
    pub stake: &'a Stake,
    /// The epoch whose weights the committee carries.
    pub epoch: Epoch,
}

impl<Stake, Validator, Amount, Epoch> ValidatorSorts for EpochCommittee<'_, Stake, Epoch>
where
    Stake: WeightSourceAlg<Validator = Validator, Amount = Amount>,
{
    type Validator = Validator;
    type Weight = Amount;
}

impl<Stake, Validator, Amount, Epoch> ValidatorSet for EpochCommittee<'_, Stake, Epoch>
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
    fn validators(&self) -> Vec<Validator> {
        self.stake.stake_weights(&self.epoch).into_iter().map(|(validator, _)| validator).collect()
    }
}

impl<Stake, Validator, Amount, Epoch> ValidatorWeight for EpochCommittee<'_, Stake, Epoch>
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
    fn weight(&self, validator: &Validator) -> Amount {
        self.stake.stake_weight(validator, &self.epoch)
    }
}

/// Derives an epoch's committee and its quorum from epoch weights and a threshold.
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
    /// Returns the committee of `epoch`, weighted by stake.
    fn stake_committee(&self, epoch: &Epoch) -> EpochCommittee<'_, Self, Epoch> {
        EpochCommittee { stake: self, epoch: epoch.clone() }
    }

    /// Says whether the voters' combined weight in the epoch decides; a repeated voter counts once.
    fn stake_quorum(&self, epoch: &Epoch, voters: &[Validator]) -> bool
    where
        This: QuorumThreshold,
    {
        self.stake_committee(epoch).weighted_quorum(self, voters.iter().cloned())
    }
}

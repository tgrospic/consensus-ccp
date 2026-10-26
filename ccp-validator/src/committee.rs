//! Defines committee membership, order, and voting weight.
use crate::{QuorumThreshold, QuorumThresholdExt, ValidatorSorts};
use alux_ext::ext;
use std::{collections::HashSet, hash::Hash};

/// Observes the members of the chosen validator sort.
pub trait ValidatorSet: ValidatorSorts {
    /// Returns each participating validator exactly once, in no semantic order.
    fn validators(&self) -> Vec<Self::Validator>;
}

/// Exposes the semantic validator order used by deterministic leader policies.
pub trait CommitteeOrder: ValidatorSorts {
    /// Returns every validator exactly once in leader-selection order.
    fn committee_order(&self) -> Vec<Self::Validator>;
}

/// Exposes the voting weight of each committee member.
pub trait ValidatorWeight: ValidatorSorts {
    /// Returns the voting weight of `validator`.
    fn weight(&self, validator: &Self::Validator) -> Self::Weight;
}

/// Derives committee weight and weighted support.
#[ext(name = WeightedCommitteeExt)]
pub impl<This, V, W> This
where
    This: ValidatorSet<Validator = V> + ValidatorWeight<Weight = W>,
    W: Into<u128>,
{
    /// Returns the total voting weight of the committee.
    fn total_weight(&self) -> u128 {
        self.validators().iter().map(|validator| self.weight(validator).into()).fold(0, u128::saturating_add)
    }

    /// Returns the weight of the distinct committee members among `supporters`.
    ///
    /// Repeated supporters count once and validators outside the committee count nothing.
    fn support_weight<I>(&self, supporters: I) -> u128
    where
        I: IntoIterator<Item = V>,
        V: Eq + Hash,
    {
        let validators = self.validators();
        supporters
            .into_iter()
            .filter(|validator| validators.contains(validator))
            .collect::<HashSet<_>>()
            .iter()
            .map(|validator| self.weight(validator).into())
            .fold(0, u128::saturating_add)
    }

    /// Reports whether `supporters` carry a quorum of committee weight under `threshold`.
    fn weighted_quorum<Threshold, I>(&self, threshold: &Threshold, supporters: I) -> bool
    where
        Threshold: QuorumThreshold,
        I: IntoIterator<Item = V>,
        V: Eq + Hash,
    {
        threshold.meets_quorum(self.support_weight(supporters), self.total_weight())
    }
}

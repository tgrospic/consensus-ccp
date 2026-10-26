//! Defines attributed validator support and its weighted quorums.
use alux_ext::ext;
use ccp_validator::{
    QuorumThreshold, TwoThirdsQuorum, ValidatorSet, ValidatorSorts, ValidatorWeight, WeightedCommitteeExt,
};
use std::hash::Hash;

/// Exposes a typed validator support relation for one evidence proposition.
pub trait ValidatorEvidence<K, T>: ValidatorSorts {
    /// Reports whether `validator` supports `target` under evidence kind `kind`.
    fn supports(&self, kind: &K, validator: &Self::Validator, target: &T) -> bool;
}

/// Identifies ordinary candidate-support evidence.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct CandidateSupport;

/// Derives typed validator quorums from a weighted committee and evidence relation.
#[ext(name = ValidatorQuorumExt)]
pub impl<This, V, W> This
where
    This: ValidatorSet<Validator = V> + ValidatorWeight<Weight = W>,
    V: Eq + Hash,
    W: Into<u128>,
{
    /// Reports whether the committee weight supporting `target` meets `policy`.
    fn has_quorum<Policy, K, T>(&self, policy: &Policy, kind: &K, target: &T) -> bool
    where
        This: ValidatorEvidence<K, T>,
        Policy: QuorumThreshold,
    {
        let supporters = self.validators().into_iter().filter(|validator| self.supports(kind, validator, target));
        self.weighted_quorum(policy, supporters.collect::<Vec<_>>())
    }
}

/// Derives ordinary candidate legitimacy from support weight strictly greater than 2/3.
#[ext(name = CandidateLegitimacyExt)]
pub impl<This, V, W, C> This
where
    This: ValidatorSet<Validator = V> + ValidatorWeight<Weight = W> + ValidatorEvidence<CandidateSupport, C>,
    V: Eq + Hash,
    W: Into<u128>,
{
    /// Reports whether `candidate` has support weight strictly greater than 2/3.
    fn two_thirds_legitimate(&self, candidate: &C) -> bool {
        self.has_quorum(&TwoThirdsQuorum, &CandidateSupport, candidate)
    }
}

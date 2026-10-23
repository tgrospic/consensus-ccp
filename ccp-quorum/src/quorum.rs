//! Defines validator quorum meaning shared by Byzantine protocol families.
use crate::ValidatorSorts;
use alux_ext::ext;

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

/// Exposes a typed validator support relation for one evidence proposition.
pub trait ValidatorEvidence<K, T>: ValidatorSorts {
    /// Reports whether `validator` supports `target` under evidence kind `kind`.
    fn supports(&self, kind: &K, validator: &Self::Validator, target: &T) -> bool;
}

/// Exposes a strict quorum fraction independently from the proposition being supported.
pub trait QuorumThreshold {
    /// Returns the numerator of the required supported fraction.
    fn threshold_numerator(&self) -> u64;

    /// Returns the denominator of the required supported fraction.
    fn threshold_denominator(&self) -> u64;
}

/// Represents the Byzantine quorum threshold strictly greater than 2/3.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TwoThirdsQuorum;

impl QuorumThreshold for TwoThirdsQuorum {
    fn threshold_numerator(&self) -> u64 {
        2
    }

    fn threshold_denominator(&self) -> u64 {
        3
    }
}

/// Identifies ordinary candidate-support evidence.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct CandidateSupport;

/// Derives quorum arithmetic from a threshold policy.
#[ext(name = QuorumThresholdExt)]
pub impl<This> This
where
    This: QuorumThreshold,
{
    /// Reports whether `supported` out of `total` participants strictly exceeds this threshold.
    fn meets_quorum(&self, supported: usize, total: usize) -> bool {
        let denominator = u128::from(self.threshold_denominator());
        let numerator = u128::from(self.threshold_numerator());
        let supported = supported as u128;
        let total = total as u128;
        total != 0
            && denominator != 0
            && numerator != 0
            && numerator <= denominator
            && supported.saturating_mul(denominator) > total.saturating_mul(numerator)
    }

    /// Returns the minimum participant count required to reach this quorum.
    fn quorum_size(&self, total: usize) -> usize {
        let denominator = u128::from(self.threshold_denominator());
        let numerator = u128::from(self.threshold_numerator());
        if denominator == 0 || numerator == 0 || numerator > denominator {
            return usize::MAX;
        }
        if total == 0 {
            return 0;
        }
        let required = (total as u128).saturating_mul(numerator) / denominator;
        let required = required.saturating_add(1);
        usize::try_from(required).unwrap_or(usize::MAX)
    }
}

/// Derives typed validator quorums from a validator set and evidence relation.
#[ext(name = ValidatorQuorumExt)]
pub impl<This> This
where
    This: ValidatorSet,
{
    /// Reports whether `target` has a quorum for the specified evidence kind.
    fn has_quorum<Policy, K, V, T>(&self, policy: &Policy, kind: &K, target: &T) -> bool
    where
        This: ValidatorSet<Validator = V> + ValidatorEvidence<K, T>,
        Policy: QuorumThreshold,
    {
        let validators = self.validators();
        let supported = validators.iter().filter(|validator| self.supports(kind, validator, target)).count();
        policy.meets_quorum(supported, validators.len())
    }
}

/// Derives ordinary candidate legitimacy from support strictly greater than 2/3.
#[ext(name = CandidateLegitimacyExt)]
pub impl<This, V, C> This
where
    This: ValidatorSet<Validator = V> + ValidatorEvidence<CandidateSupport, C>,
{
    /// Reports whether `candidate` has support strictly greater than 2/3.
    fn two_thirds_legitimate(&self, candidate: &C) -> bool {
        self.has_quorum(&TwoThirdsQuorum, &CandidateSupport, candidate)
    }
}

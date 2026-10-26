//! Defines strict quorum fractions over committee weight.
use alux_ext::ext;

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

/// Derives quorum arithmetic over weights from a threshold policy.
#[ext(name = QuorumThresholdExt)]
pub impl<This> This
where
    This: QuorumThreshold,
{
    /// Reports whether `supported` out of `total` weight strictly exceeds this threshold, or is the
    /// whole committee weight.
    ///
    /// The whole weight is always a quorum, so the threshold 1/1 means unanimity, as in a genesis
    /// committee where one validator holds all the stake.
    fn meets_quorum(&self, supported: u128, total: u128) -> bool {
        let denominator = u128::from(self.threshold_denominator());
        let numerator = u128::from(self.threshold_numerator());
        total != 0
            && denominator != 0
            && numerator != 0
            && numerator <= denominator
            && (supported >= total || supported.saturating_mul(denominator) > total.saturating_mul(numerator))
    }

    /// Returns the least weight out of `total` that reaches this quorum.
    fn quorum_size(&self, total: u128) -> u128 {
        let denominator = u128::from(self.threshold_denominator());
        let numerator = u128::from(self.threshold_numerator());
        if denominator == 0 || numerator == 0 || numerator > denominator {
            return u128::MAX;
        }
        if total == 0 {
            return 0;
        }
        (total.saturating_mul(numerator) / denominator).saturating_add(1).min(total)
    }

    /// Returns how much of `total` weight may fail while the rest still reaches this quorum.
    fn fault_bound(&self, total: u128) -> u128 {
        total.saturating_sub(self.quorum_size(total))
    }

    /// Reports whether any two quorums of this threshold share committee weight.
    fn quorums_intersect(&self) -> bool {
        u128::from(self.threshold_numerator()) * 2 >= u128::from(self.threshold_denominator())
    }
}

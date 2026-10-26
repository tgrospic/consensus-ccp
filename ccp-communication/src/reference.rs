//! Defines the reference round-advance policy.
use crate::AdvancePolicy;
use ccp_validator::QuorumThreshold;
use derive_new::new as New;

/// Advances a round once a strict threshold fraction of validators covers each round in a window.
#[derive(Debug, Clone, Copy, New, PartialEq, Eq)]
pub struct QuorumAdvancePolicy {
    threshold_numerator: u64,
    threshold_denominator: u64,
    window: u64,
}

/// Builds the reference advance policy: strictly more than 2/3 of validators in the previous round.
pub fn quorum_advance_policy() -> QuorumAdvancePolicy {
    QuorumAdvancePolicy::new(2, 3, 1)
}

impl AdvancePolicy for QuorumAdvancePolicy {
    fn window(&self) -> u64 {
        self.window
    }
}

impl QuorumThreshold for QuorumAdvancePolicy {
    fn threshold_numerator(&self) -> u64 {
        self.threshold_numerator
    }

    fn threshold_denominator(&self) -> u64 {
        self.threshold_denominator
    }
}

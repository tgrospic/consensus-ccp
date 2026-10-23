//! Defines reference leader-selection policies and the unrestricted reference policy.
use crate::{LeaderSelection, LeaderSelectionPolicy, Policy};
use ccp_core::{AllCandidates, SlotNumber, Unbounded};
use derive_new::new as New;

/// The unrestricted CCP reference: maximal candidates and no direct-horizon bound.
pub type ReferencePolicy = Policy<AllCandidates, Unbounded, AllLeaderSlots>;

/// Selects every validator's message slot as a leader candidate in each round.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AllLeaderSlots;

/// Selects one round-robin validator on a periodic round cadence; `new(0, 1)` selects every round.
#[derive(Debug, Clone, Copy, New, PartialEq, Eq)]
pub struct PeriodicRoundRobin {
    first: u64,
    period: u64,
}

/// Builds the unrestricted reference policy.
pub fn reference_policy() -> ReferencePolicy {
    Policy::new(AllCandidates, Unbounded, AllLeaderSlots)
}

impl<S, V> LeaderSelectionPolicy<S, V> for AllLeaderSlots
where
    V: Clone,
{
    fn select_leaders(&self, _round: &S, validators: &[V]) -> LeaderSelection<V> {
        LeaderSelection::Selected(validators.to_vec())
    }
}

impl<S, V> LeaderSelectionPolicy<S, V> for PeriodicRoundRobin
where
    S: SlotNumber,
    V: Clone,
{
    fn select_leaders(&self, round: &S, validators: &[V]) -> LeaderSelection<V> {
        let position = round.slot_number();
        if self.period == 0 || position < self.first || !(position - self.first).is_multiple_of(self.period) {
            return LeaderSelection::NotScheduled;
        }

        let index = usize::try_from((position - self.first) / self.period).unwrap_or(usize::MAX);
        validators
            .get(index % validators.len().max(1))
            .cloned()
            .map_or(LeaderSelection::Unresolved, |validator| LeaderSelection::Selected(vec![validator]))
    }
}

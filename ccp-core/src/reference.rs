//! Defines reference candidate restrictions, evidence horizons, and the numeric slot interpretation.
use crate::{CandidateRestriction, EvidenceHorizon, FollowingBoundary, SlotNumber};
use derive_new::new as New;

/// Keeps every candidate from the communication carrier.
#[derive(Debug, Clone, Copy, Default)]
pub struct AllCandidates;

/// Keeps candidates only on a periodic slot cadence.
#[derive(Debug, Clone, Copy, New, PartialEq, Eq)]
pub struct EveryNthSlot {
    first: u64,
    period: u64,
}

/// Preserves the entire observed communication carrier.
#[derive(Debug, Clone, Copy, Default)]
pub struct Unbounded;

/// Restricts direct evidence to a fixed inclusive distance from the candidate slot, which also
/// places the following decision boundary.
#[derive(Debug, Clone, Copy, New, PartialEq, Eq)]
pub struct FixedDistance {
    distance: u64,
}

impl<S, C> CandidateRestriction<S, C> for AllCandidates {
    fn includes_candidate(&self, _slot: &S, _candidate: &C) -> bool {
        true
    }
}

impl<S, C> CandidateRestriction<S, C> for EveryNthSlot
where
    S: SlotNumber,
{
    fn includes_candidate(&self, slot: &S, _candidate: &C) -> bool {
        self.period != 0
            && slot.slot_number() >= self.first
            && (slot.slot_number() - self.first).is_multiple_of(self.period)
    }
}

impl<S> EvidenceHorizon<S> for Unbounded {
    fn includes_evidence(&self, _candidate_slot: &S, _evidence_slot: &S) -> bool {
        true
    }
}

impl FollowingBoundary<u64> for FixedDistance {
    fn following_boundary(&self, slot: &u64) -> u64 {
        slot.saturating_add(self.distance)
    }
}

impl<S> EvidenceHorizon<S> for FixedDistance
where
    S: SlotNumber,
{
    fn includes_evidence(&self, candidate_slot: &S, evidence_slot: &S) -> bool {
        let candidate = candidate_slot.slot_number();
        let evidence = evidence_slot.slot_number();
        evidence >= candidate && evidence - candidate <= self.distance
    }
}

impl SlotNumber for u64 {
    fn slot_number(&self) -> u64 {
        *self
    }
}

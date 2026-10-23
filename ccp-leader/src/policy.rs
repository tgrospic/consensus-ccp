use crate::{LeaderSelection, LeaderSelectionPolicy};
use ccp_core::{CandidateRestriction, EvidenceHorizon, FollowingBoundary};
use derive_new::new as New;

/// A policy is the product of candidate, horizon, and leader-selection policies.
#[derive(Debug, Clone, New, PartialEq, Eq)]
pub struct Policy<Candidates, Horizon, Leaders> {
    candidates: Candidates,
    horizon: Horizon,
    leaders: Leaders,
}

impl<S, C, Candidates, Horizon, Leaders> CandidateRestriction<S, C> for Policy<Candidates, Horizon, Leaders>
where
    Candidates: CandidateRestriction<S, C>,
{
    fn includes_candidate(&self, slot: &S, candidate: &C) -> bool {
        self.candidates.includes_candidate(slot, candidate)
    }
}

impl<S, Candidates, Horizon, Leaders> EvidenceHorizon<S> for Policy<Candidates, Horizon, Leaders>
where
    Horizon: EvidenceHorizon<S>,
{
    fn includes_evidence(&self, candidate_slot: &S, evidence_slot: &S) -> bool {
        self.horizon.includes_evidence(candidate_slot, evidence_slot)
    }
}

impl<S, Candidates, Horizon, Leaders> FollowingBoundary<S> for Policy<Candidates, Horizon, Leaders>
where
    Horizon: FollowingBoundary<S>,
{
    fn following_boundary(&self, slot: &S) -> S {
        self.horizon.following_boundary(slot)
    }
}

impl<S, V, Candidates, Horizon, Leaders> LeaderSelectionPolicy<S, V> for Policy<Candidates, Horizon, Leaders>
where
    Leaders: LeaderSelectionPolicy<S, V>,
{
    fn select_leaders(&self, slot: &S, validators: &[V]) -> LeaderSelection<V> {
        self.leaders.select_leaders(slot, validators)
    }
}

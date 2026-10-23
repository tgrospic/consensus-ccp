//! Defines independent candidate and evidence-horizon restrictions.
use crate::CandidateSpace;
use alux_ext::ext;

/// Gives a slot a stable numeric position for cadence and distance policies.
pub trait SlotNumber {
    /// Returns the slot position.
    fn slot_number(&self) -> u64;
}

/// Restricts which communication-carrier candidates remain eligible anchors.
pub trait CandidateRestriction<S, C> {
    /// Reports whether `candidate` remains eligible at `slot`.
    fn includes_candidate(&self, slot: &S, candidate: &C) -> bool;
}

/// Restricts which evidence slots are visible to a direct decision.
pub trait EvidenceHorizon<S> {
    /// Reports whether evidence at `evidence_slot` is visible from `candidate_slot`.
    fn includes_evidence(&self, candidate_slot: &S, evidence_slot: &S) -> bool;
}

/// Derives eligible candidates from a candidate restriction.
#[ext(name = CandidateRestrictionExt)]
pub impl<This, S, C> This
where
    This: CandidateRestriction<S, C>,
{
    /// Applies this restriction to a carrier without changing the carrier itself.
    fn eligible_candidates<Space>(&self, carrier: &Space, slot: &S) -> Vec<C>
    where
        Space: CandidateSpace<Slot = S, Candidate = C>,
    {
        carrier.candidates(slot).into_iter().filter(|candidate| self.includes_candidate(slot, candidate)).collect()
    }
}

/// Derives visible slots from an evidence-horizon restriction.
#[ext(name = EvidenceHorizonExt)]
pub impl<This, S> This
where
    This: EvidenceHorizon<S>,
{
    /// Applies this horizon to the slots retained by the carrier.
    fn visible_slots<Space, C>(&self, carrier: &Space, candidate_slot: &S) -> Vec<S>
    where
        Space: CandidateSpace<Slot = S, Candidate = C>,
    {
        carrier.slots().into_iter().filter(|slot| self.includes_evidence(candidate_slot, slot)).collect()
    }
}

//! Defines protocol-independent candidate, closure, and decision meaning.
use crate::CandidateSorts;
use alux_ext::ext;

/// A closed decision for one candidate slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Closed<C> {
    /// Commits the invariant candidate.
    Commit(C),
    /// Closes the slot without a candidate.
    Skip,
}

/// The observable decision state of one candidate slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision<S, C> {
    /// Commits a candidate.
    Commit(C),
    /// Closes a slot without a candidate.
    Skip(S),
    /// Retains a slot whose result can still change.
    Undecided(S),
}

impl<S, C> Decision<S, C> {
    /// Reports whether the decision has crossed the finalization frontier.
    pub fn is_closed(&self) -> bool {
        !matches!(self, Self::Undecided(_))
    }
}

/// Observes candidates in the chosen candidate sorts.
pub trait CandidateSpace: CandidateSorts {
    /// Returns candidate slots in their consensus order.
    fn slots(&self) -> Vec<Self::Slot>;

    /// Returns every proposal occupying `slot`.
    fn candidates(&self, slot: &Self::Slot) -> Vec<Self::Candidate>;

    /// Returns the slot occupied by `candidate`.
    fn slot_of(&self, candidate: &Self::Candidate) -> Self::Slot;
}

/// Exposes the semantic closure oracle admitted by the unrestricted reference policy.
///
/// `None` means that lawful communication expansions can still change the slot observation.
pub trait SemanticClosure: CandidateSorts {
    /// Returns the invariant outcome once the full retained carrier determines it.
    fn closed_outcome(&self, slot: &Self::Slot) -> Option<Closed<Self::Candidate>>;
}

/// Exposes the evidence boundary one decision period after a candidate slot.
pub trait FollowingBoundary<S> {
    /// Returns the end of the following decision period.
    fn following_boundary(&self, slot: &S) -> S;
}

/// Derives observable decisions from unrestricted semantic closure.
#[ext(name = ReferenceDecisionExt)]
pub impl<This, S, C> This
where
    This: SemanticClosure<Slot = S, Candidate = C>,
    S: Clone,
{
    /// Decides a slot by semantic closure over the unrestricted semantic carrier.
    fn reference_decision(&self, slot: &S) -> Decision<S, C> {
        match self.closed_outcome(slot) {
            Some(Closed::Commit(candidate)) => Decision::Commit(candidate),
            Some(Closed::Skip) => Decision::Skip(slot.clone()),
            None => Decision::Undecided(slot.clone()),
        }
    }
}

//! Defines leader selection by round and authenticated candidate authorship.
use ccp_core::CandidateSorts;
use ccp_quorum::ValidatorSorts;

/// Describes whether and which validators are selected as leaders for one round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaderSelection<V> {
    /// Indicates that the cadence does not schedule a leader in this round.
    NotScheduled,
    /// Indicates that communication history has not revealed the leader yet.
    Unresolved,
    /// Selects the validators whose message slots are leader candidates.
    Selected(Vec<V>),
}

/// Selects leader validators from a round and an ordered validator set.
pub trait LeaderSelectionPolicy<R, V> {
    /// Returns the leader selection observed in `round`.
    fn select_leaders(&self, round: &R, validators: &[V]) -> LeaderSelection<V>;
}

/// Exposes the validator that authored a candidate.
pub trait CandidateAuthor: CandidateSorts + ValidatorSorts {
    /// Returns the candidate's authenticated author when known.
    fn candidate_author(&self, candidate: &Self::Candidate) -> Option<Self::Validator>;
}

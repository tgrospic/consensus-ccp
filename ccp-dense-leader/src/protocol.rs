//! Specifies an unbounded leader-collapse policy with legitimacy strictly greater than 2/3.
use alux_ext::ext;
use ccp_communication::{JustificationRelation, PreviousRoundWitnesses};
use ccp_core::CandidateSorts;
use ccp_core::{AllCandidates, CandidateSpace, Decision, Unbounded};
use ccp_leader::{AllLeaderSlots, Policy};
use ccp_quorum::{CandidateLegitimacyExt, CandidateSupport, QuorumOf, ValidatorEvidence};
use ccp_validator::{TwoThirdsQuorum, ValidatorSet, ValidatorWeight};
use std::hash::Hash;

/// Names a quorum of justification as dense legitimacy.
pub type DenseLegitimacy<'a, Carrier> =
    QuorumOf<PreviousRoundWitnesses<'a, Carrier>, JustificationRelation<'a, Carrier>, TwoThirdsQuorum>;

/// Describes dense leader scheduling and an unbounded horizon for comparison scenarios.
pub type DenseLeaderPolicy = Policy<AllCandidates, Unbounded, AllLeaderSlots>;

/// Builds the comparison policy used to construct dense legitimacy evidence.
pub fn dense_leader_policy() -> DenseLeaderPolicy {
    Policy::new(AllCandidates, Unbounded, AllLeaderSlots)
}

/// Exposes the designated leader candidate for each dense legitimacy slot.
pub trait SlotLeader: CandidateSorts {
    /// Returns the designated leader occupying `slot`, when one was produced.
    fn slot_leader(&self, slot: &Self::Slot) -> Option<Self::Candidate>;
}

/// Derives leader finalization from a separately interpreted legitimacy view.
#[ext(name = DenseLeaderEvidenceCollapseExt)]
pub impl<This, S, C, V, W> This
where
    This: CandidateSpace<Slot = S, Candidate = C> + SlotLeader,
    S: Clone + Ord,
    C: Clone + Eq + Hash,
{
    /// Returns every leader accepted by `evidence`, optionally above an existing frontier.
    fn dense_leader_finalizations_from<Evidence>(&self, evidence: &Evidence, frontier: Option<&S>) -> Vec<C>
    where
        Evidence: ValidatorEvidence<CandidateSupport, C> + ValidatorSet<Validator = V> + ValidatorWeight<Weight = W>,
        V: Eq + Hash,
        W: Into<u128>,
    {
        self.slots()
            .into_iter()
            .filter(|slot| frontier.is_none_or(|frontier| slot > frontier))
            .filter_map(|slot| self.slot_leader(&slot))
            .filter(|candidate| evidence.two_thirds_legitimate(candidate))
            .collect()
    }

    /// Returns every leader accepted by `evidence` above `frontier`, oldest first.
    fn dense_leader_finalizations<Evidence>(&self, evidence: &Evidence, frontier: &S) -> Vec<C>
    where
        Evidence: ValidatorEvidence<CandidateSupport, C> + ValidatorSet<Validator = V> + ValidatorWeight<Weight = W>,
        V: Eq + Hash,
        W: Into<u128>,
    {
        self.dense_leader_finalizations_from(evidence, Some(frontier))
    }

    /// Classifies a slot from all leaders accepted in the current history.
    fn dense_leader_history_decision<Evidence>(&self, evidence: &Evidence, slot: &S) -> Decision<S, C>
    where
        Evidence: ValidatorEvidence<CandidateSupport, C> + ValidatorSet<Validator = V> + ValidatorWeight<Weight = W>,
        V: Eq + Hash,
        W: Into<u128>,
    {
        self.dense_leader_decision_from(evidence, slot, None)
    }

    /// Classifies a slot from the leaders accepted by `evidence`.
    fn dense_leader_decision<Evidence>(&self, evidence: &Evidence, slot: &S, frontier: &S) -> Decision<S, C>
    where
        Evidence: ValidatorEvidence<CandidateSupport, C> + ValidatorSet<Validator = V> + ValidatorWeight<Weight = W>,
        V: Eq + Hash,
        W: Into<u128>,
    {
        self.dense_leader_decision_from(evidence, slot, Some(frontier))
    }

    /// Classifies a slot from accepted leaders relative to an optional existing frontier.
    fn dense_leader_decision_from<Evidence>(
        &self,
        evidence: &Evidence,
        slot: &S,
        frontier: Option<&S>,
    ) -> Decision<S, C>
    where
        Evidence: ValidatorEvidence<CandidateSupport, C> + ValidatorSet<Validator = V> + ValidatorWeight<Weight = W>,
        V: Eq + Hash,
        W: Into<u128>,
    {
        let leaders = self.dense_leader_finalizations_from(evidence, frontier);
        leaders.iter().find(|candidate| &self.slot_of(candidate) == slot).cloned().map_or_else(
            || {
                leaders.last().map_or_else(
                    || Decision::Undecided(slot.clone()),
                    |anchor| {
                        if frontier.is_none_or(|frontier| slot > frontier) && slot <= &self.slot_of(anchor) {
                            Decision::Skip(slot.clone())
                        } else {
                            Decision::Undecided(slot.clone())
                        }
                    },
                )
            },
            Decision::Commit,
        )
    }
}

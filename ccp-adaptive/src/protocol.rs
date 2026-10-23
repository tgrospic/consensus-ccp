//! Specifies pipelined adaptive-horizon collapse and justification-history fringe projection.
use alux_ext::ext;
use ccp_communication::{JustificationRelation, PreviousRoundWitnesses};
use ccp_core::CandidateSorts;
use ccp_core::{
    AllCandidates, CandidateRestriction, CandidateSpace, Decision, FixedDistance, FollowingBoundary,
    PredecessorChainExt,
};
use ccp_leader::{CandidateAuthor, LeaderSelection, LeaderSelectionPolicy, PeriodicRoundRobin, Policy};
use ccp_quorum::{
    CandidateLegitimacyExt, CandidateSupport, CommitteeOrder, QuorumOf, TwoThirdsQuorum, ValidatorEvidence,
    ValidatorSet,
};
use std::collections::HashSet;
use std::hash::Hash;

/// Names a quorum of justification as ratification in Adaptive's vocabulary.
pub type AdaptiveRatification<'a, Carrier> =
    QuorumOf<PreviousRoundWitnesses<'a, Carrier>, JustificationRelation<'a, Carrier>, TwoThirdsQuorum>;

/// Names the general horizon-evidence capability in Adaptive's vocabulary.
pub use ccp_core::HorizonEvidence as AdaptiveEvidence;

/// Exposes the latest earlier candidate certified in `anchor`'s view.
pub trait CertifiedPredecessor: CandidateSorts {
    /// Returns the latest earlier certified candidate above `frontier`.
    fn previous_certified(&self, anchor: &Self::Candidate, frontier: &Self::Slot) -> Option<Self::Candidate>;
}

/// Exposes the strict justification past used for adaptive fringe projection.
pub trait JustificationPast: CandidateSorts {
    /// Returns the strict justification past of `anchor`.
    fn justification_past(&self, anchor: &Self::Candidate) -> HashSet<Self::Candidate>;
}

/// Composes pipelined candidate retention and the adaptive horizon.
pub type AdaptivePolicy = Policy<AllCandidates, FixedDistance, PeriodicRoundRobin>;

/// Builds the pipelined adaptive policy with the given horizon upper bound.
pub fn adaptive_policy(upper_bound: u64) -> AdaptivePolicy {
    Policy::new(AllCandidates, FixedDistance::new(upper_bound), PeriodicRoundRobin::new(0, 1))
}

/// Derives the candidate-dependent direct horizon and direct commit predicate.
#[ext(name = AdaptiveDirectExt)]
pub impl<This, S, C, V> This
where
    This: AdaptiveEvidence<S, C>
        + CandidateSpace<Slot = S, Candidate = C>
        + ValidatorEvidence<CandidateSupport, C>
        + ValidatorSet<Validator = V>,
    S: Clone + Ord,
    C: Clone,
{
    /// Derives the candidate-specific horizon boundary.
    fn adaptive_horizon<Schedule>(&self, candidate: &C, schedule: &Schedule) -> S
    where
        Schedule: FollowingBoundary<S>,
    {
        schedule.following_boundary(&self.slot_of(candidate))
    }

    /// Reports whether the candidate survives reconstruction at its adaptive horizon.
    fn adaptive_direct_commit<Schedule>(&self, candidate: &C, schedule: &Schedule) -> bool
    where
        Schedule: FollowingBoundary<S>,
    {
        self.two_thirds_legitimate(candidate)
            && self.current_matches(candidate)
            && self.matches_at(candidate, &self.adaptive_horizon(candidate, schedule))
    }
}

/// Derives direct-anchor selection from authenticated authors and the leader policy.
#[ext(name = AdaptiveLeaderExt)]
pub impl<This, S, C, V> This
where
    This: CandidateSpace<Slot = S, Candidate = C>
        + AdaptiveEvidence<S, C>
        + CandidateAuthor<Validator = V>
        + CommitteeOrder
        + ValidatorEvidence<CandidateSupport, C>
        + ValidatorSet,
    S: Clone + Ord,
    C: Clone,
    V: Clone + Eq,
{
    /// Selects the newest directly committable round-robin leader above `frontier`.
    fn adaptive_direct_anchor<Schedule>(&self, frontier: &S, schedule: &Schedule) -> Option<C>
    where
        Schedule: CandidateRestriction<S, C> + FollowingBoundary<S> + LeaderSelectionPolicy<S, V>,
    {
        let validators = self.committee_order();
        self.slots().into_iter().rev().take_while(|slot| slot > frontier).find_map(|slot| {
            let leaders = match schedule.select_leaders(&slot, &validators) {
                LeaderSelection::Selected(leaders) => leaders,
                LeaderSelection::NotScheduled | LeaderSelection::Unresolved => return None,
            };
            self.candidates(&slot)
                .into_iter()
                .filter(|candidate| schedule.includes_candidate(&slot, candidate))
                .filter(|candidate| self.candidate_author(candidate).is_some_and(|author| leaders.contains(&author)))
                .find(|candidate| self.adaptive_direct_commit(candidate, schedule))
        })
    }
}

/// Derives the recursive latest-certified anchor chain.
#[ext(name = AdaptiveIndirectExt)]
pub impl<This, S, C> This
where
    This: CertifiedPredecessor<Slot = S, Candidate = C>,
    C: Clone + Eq + Hash,
{
    /// Returns the recursive latest-certified chain, oldest first.
    fn adaptive_anchor_chain(&self, direct_anchor: &C, frontier: &S) -> Vec<C> {
        let previous = |candidate: &C| self.previous_certified(candidate, frontier);
        previous.predecessor_chain(direct_anchor.clone())
    }
}

/// Derives the complete adaptive anchor collapse from direct and indirect rules.
#[ext(name = AdaptiveCollapseExt)]
pub impl<This, S, C, V> This
where
    This: CandidateSpace<Slot = S, Candidate = C>
        + AdaptiveEvidence<S, C>
        + CandidateAuthor<Validator = V>
        + CertifiedPredecessor
        + CommitteeOrder
        + ValidatorEvidence<CandidateSupport, C>
        + ValidatorSet,
    S: Clone + Ord,
    C: Clone + Eq + Hash,
    V: Clone + Eq,
{
    /// Returns all newly finalized anchors above `frontier`, oldest first.
    fn adaptive_finalizations<Schedule>(&self, frontier: &S, schedule: &Schedule) -> Vec<C>
    where
        Schedule: CandidateRestriction<S, C> + FollowingBoundary<S> + LeaderSelectionPolicy<S, V>,
    {
        self.adaptive_direct_anchor(frontier, schedule)
            .map_or_else(Vec::new, |anchor| self.adaptive_anchor_chain(&anchor, frontier))
    }

    /// Classifies a candidate slot from the newly finalized anchor frontier.
    fn adaptive_decision<Schedule>(&self, slot: &S, frontier: &S, schedule: &Schedule) -> Decision<S, C>
    where
        Schedule: CandidateRestriction<S, C> + FollowingBoundary<S> + LeaderSelectionPolicy<S, V>,
    {
        let anchors = self.adaptive_finalizations(frontier, schedule);
        anchors.iter().find(|candidate| &self.slot_of(candidate) == slot).cloned().map_or_else(
            || {
                anchors.last().map_or_else(
                    || Decision::Undecided(slot.clone()),
                    |anchor| {
                        if slot > frontier && slot <= &self.slot_of(anchor) {
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

/// Derives the half-open justification-past fringe projection.
#[ext(name = AdaptiveFringeExt)]
pub impl<This, C> This
where
    This: JustificationPast<Candidate = C>,
    C: Clone + Eq + Hash,
{
    /// Returns the strict justification past not already emitted by earlier fringes.
    fn adaptive_fringe(&self, anchor: &C, finalized: &HashSet<C>) -> HashSet<C> {
        self.justification_past(anchor).difference(finalized).cloned().collect()
    }
}

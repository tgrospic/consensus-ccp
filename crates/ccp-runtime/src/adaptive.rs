//! Interprets Adaptive evidence from one observer's reachable communication history.
use crate::CommunicationHistory;
use ccp_adaptive::{AdaptiveEvidence, CertifiedPredecessor, JustificationPast};
use ccp_communication::{
    JustificationRelation, MessageCarrier, MessageCarrierExt, MessageJustificationExt, MessageSlot,
    ObservedHorizonMatchEvidence, PreviousRoundWitnesses,
};
use ccp_core::{CandidateSorts, CandidateSpace, EvidenceRelation, FollowingBoundary};
use ccp_leader::{CandidateAuthor, LeaderSelection, LeaderSelectionPolicy};
use ccp_quorum::{CandidateSupport, QuorumOf, ValidatorEvidence};
use ccp_validator::{CommitteeOrder, TwoThirdsQuorum, ValidatorSet, ValidatorSorts, ValidatorWeight};
use derive_new::new as New;
use std::{collections::HashSet, hash::Hash};

/// Interprets Adaptive capabilities from one observer's causal history.
#[derive(Debug, Clone, Copy, New)]
pub struct AdaptiveDagView<'a, V, C, Schedule> {
    history: &'a CommunicationHistory<V, C>,
    schedule: &'a Schedule,
    observer: &'a C,
}

impl<V, C, Schedule> AdaptiveDagView<'_, V, C, Schedule>
where
    Schedule: LeaderSelectionPolicy<u64, V>,
    V: Clone + Eq + Ord,
    C: Clone + Eq + Hash + Ord,
{
    /// Returns the unique observed candidate of a scheduled leader slot.
    fn leader(&self, slot: &MessageSlot<V>) -> Option<C> {
        let committee = self.history.committee_order();
        let selected = matches!(
            self.schedule.select_leaders(&slot.round, &committee),
            LeaderSelection::Selected(leaders) if leaders.contains(&slot.validator)
        );
        let mut candidates = self.candidates(slot).into_iter().filter(|_| selected);
        candidates.next().filter(|_| candidates.next().is_none())
    }

    /// Reports whether the observed messages up to `round` show `target` uncontested in its slot.
    fn uncontested_until(&self, target: &C, round: u64) -> bool {
        let cut = self
            .history
            .messages()
            .into_iter()
            .filter(|message| self.history.justification_observes(self.observer, message))
            .filter(|message| self.history.slot_of(message).round <= round)
            .collect::<Vec<_>>();
        let contested = |conflict: &C| cut.iter().any(|message| self.history.justification_observes(message, conflict));
        cut.contains(target)
            && self
                .history
                .slot_messages(&self.history.slot_of(target))
                .iter()
                .filter(|candidate| *candidate != target)
                .all(|conflict| !contested(conflict))
    }
}

impl<V, C, Schedule> CandidateSorts for AdaptiveDagView<'_, V, C, Schedule> {
    type Slot = MessageSlot<V>;
    type Candidate = C;
}

impl<V, C, Schedule> CandidateSpace for AdaptiveDagView<'_, V, C, Schedule>
where
    V: Clone + Eq + Ord,
    C: Clone + Eq + Hash + Ord,
{
    fn slots(&self) -> Vec<MessageSlot<V>> {
        self.history.message_slots().into_iter().filter(|slot| !self.candidates(slot).is_empty()).collect()
    }

    fn candidates(&self, slot: &MessageSlot<V>) -> Vec<C> {
        self.history
            .slot_messages(slot)
            .into_iter()
            .filter(|candidate| self.history.justification_observes(self.observer, candidate))
            .collect()
    }

    fn slot_of(&self, candidate: &C) -> MessageSlot<V> {
        self.history.slot_of(candidate)
    }
}

impl<V, C, Schedule> ValidatorSorts for AdaptiveDagView<'_, V, C, Schedule> {
    type Validator = V;
    type Weight = u64;
}

impl<V, C, Schedule> ValidatorSet for AdaptiveDagView<'_, V, C, Schedule>
where
    V: Clone,
{
    fn validators(&self) -> Vec<V> {
        self.history.validators()
    }
}

impl<V, C, Schedule> ValidatorWeight for AdaptiveDagView<'_, V, C, Schedule> {
    fn weight(&self, validator: &V) -> u64 {
        self.history.weight(validator)
    }
}

impl<V, C, Schedule> CommitteeOrder for AdaptiveDagView<'_, V, C, Schedule>
where
    V: Clone,
{
    fn committee_order(&self) -> Vec<V> {
        self.history.committee_order()
    }
}

impl<V, C, Schedule> CandidateAuthor for AdaptiveDagView<'_, V, C, Schedule>
where
    V: Clone + Eq + Ord,
    C: Clone + Eq,
{
    fn candidate_author(&self, candidate: &C) -> Option<V> {
        self.history.candidate_author(candidate)
    }
}

/// Supports a candidate by ratification at its following boundary, as the observer sees it.
impl<V, C, Schedule> ValidatorEvidence<CandidateSupport, C> for AdaptiveDagView<'_, V, C, Schedule>
where
    Schedule: FollowingBoundary<u64>,
    V: Clone + Eq + Hash + Ord,
    C: Clone + Eq + Hash + Ord,
{
    fn supports(&self, kind: &CandidateSupport, validator: &V, candidate: &C) -> bool {
        let ratification = QuorumOf::new(
            PreviousRoundWitnesses::new(self.history),
            JustificationRelation::new(self.history),
            TwoThirdsQuorum,
        );
        ObservedHorizonMatchEvidence::new(self.history, self.schedule, self.observer, ratification)
            .supports(kind, validator, candidate)
    }
}

/// Matches a candidate the observer sees uncontested in its slot, now or through the messages up
/// to a boundary round.
impl<V, C, Schedule> AdaptiveEvidence<MessageSlot<V>, C> for AdaptiveDagView<'_, V, C, Schedule>
where
    Schedule: LeaderSelectionPolicy<u64, V>,
    V: Clone + Eq + Ord,
    C: Clone + Eq + Hash + Ord,
{
    fn current_matches(&self, target: &C) -> bool {
        self.uncontested_until(target, u64::MAX)
    }

    fn matches_at(&self, target: &C, boundary: &MessageSlot<V>) -> bool {
        self.uncontested_until(target, boundary.round)
    }
}

/// Finds the latest earlier scheduled leader above the frontier that the anchor ratifies.
impl<V, C, Schedule> CertifiedPredecessor for AdaptiveDagView<'_, V, C, Schedule>
where
    Schedule: LeaderSelectionPolicy<u64, V>,
    V: Clone + Eq + Hash + Ord,
    C: Clone + Eq + Hash + Ord,
{
    fn previous_certified(&self, anchor: &C, frontier: &MessageSlot<V>) -> Option<C> {
        let anchor_slot = self.slot_of(anchor);
        let ratification = QuorumOf::new(
            PreviousRoundWitnesses::new(self.history),
            JustificationRelation::new(self.history),
            TwoThirdsQuorum,
        );
        self.slots()
            .into_iter()
            .rev()
            .filter(|slot| slot < &anchor_slot && slot > frontier)
            .filter_map(|slot| self.leader(&slot))
            .find(|candidate| ratification.relates(anchor, candidate))
    }
}

impl<V, C, Schedule> JustificationPast for AdaptiveDagView<'_, V, C, Schedule>
where
    V: Clone + Eq + Ord,
    C: Clone + Eq + Hash + Ord,
{
    fn justification_past(&self, anchor: &C) -> HashSet<C> {
        self.history
            .messages()
            .into_iter()
            .filter(|message| message != anchor && self.history.justification_observes(anchor, message))
            .collect()
    }
}

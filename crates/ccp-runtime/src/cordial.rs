//! Interprets Cordial Miners evidence from reachable communication histories.
use crate::{CommunicationHistory, ObservedSlot};
use ccp_communication::{
    JustificationRelation, MessageCarrier, MessageCarrierExt, MessageJustificationExt, MessageSlot,
    ObservedHorizonMatchEvidence, PreviousRoundWitnesses, UncontestedRelation,
};
use ccp_cordial::{CordialBlocklace, CordialCollapseExt, CordialLeader, RatifiedPredecessor};
use ccp_core::CandidateSorts;
use ccp_core::{CandidateSpace, Closed, Decision, EvidenceRelation, FollowingBoundary, HistoryCollapse, HorizonMatch};
use ccp_leader::{LeaderSelection, LeaderSelectionPolicy};
use ccp_validator::{CommitteeOrder, TwoThirdsQuorum, ValidatorSet, ValidatorSorts, ValidatorWeight};
use ccp_quorum::{CandidateSupport, QuorumOf, ValidatorEvidence};
use derive_new::new as New;
use std::hash::Hash;

/// Interprets Cordial capabilities from one observer's causal history.
#[derive(Debug, Clone, Copy, New)]
pub struct CordialDagView<'a, V, C, Schedule> {
    history: &'a CommunicationHistory<V, C>,
    schedule: &'a Schedule,
    observer: &'a C,
}

impl<V, C, Schedule> CandidateSorts for CordialDagView<'_, V, C, Schedule> {
    type Slot = MessageSlot<V>;
    type Candidate = C;
}

impl<V, C, Schedule> CandidateSpace for CordialDagView<'_, V, C, Schedule>
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

impl<V, C, Schedule> ValidatorSorts for CordialDagView<'_, V, C, Schedule> {
    type Validator = V;
    type Weight = u64;
}

impl<V, C, Schedule> ValidatorSet for CordialDagView<'_, V, C, Schedule>
where
    V: Clone,
{
    fn validators(&self) -> Vec<V> {
        self.history.validators()
    }
}

impl<V, C, Schedule> ValidatorWeight for CordialDagView<'_, V, C, Schedule> {
    fn weight(&self, validator: &V) -> u64 {
        self.history.weight(validator)
    }
}

impl<V, C, Schedule> CordialLeader for CordialDagView<'_, V, C, Schedule>
where
    Schedule: LeaderSelectionPolicy<u64, V>,
    V: Clone + Eq + Ord,
    C: Clone + Eq + Hash + Ord,
{
    fn cordial_leader(&self, slot: &MessageSlot<V>) -> Option<C> {
        let committee = self.history.committee_order();
        let selected = matches!(
            self.schedule.select_leaders(&slot.round, &committee),
            LeaderSelection::Selected(leaders) if leaders.contains(&slot.validator)
        );
        if !selected {
            return None;
        }
        let mut candidates = self.candidates(slot).into_iter();
        candidates.next().filter(|_| candidates.next().is_none())
    }
}

impl<V, C, Schedule> ValidatorEvidence<CandidateSupport, C> for CordialDagView<'_, V, C, Schedule>
where
    Schedule: FollowingBoundary<u64>,
    V: Clone + Eq + Hash + Ord,
    C: Clone + Eq + Hash + Ord,
{
    fn supports(&self, kind: &CandidateSupport, validator: &V, candidate: &C) -> bool {
        let observation = JustificationRelation::new(self.history);
        let approval = UncontestedRelation::new(self.history, observation);
        let ratification = QuorumOf::new(PreviousRoundWitnesses::new(self.history), approval, TwoThirdsQuorum);
        ObservedHorizonMatchEvidence::new(self.history, self.schedule, self.observer, ratification)
            .supports(kind, validator, candidate)
    }
}

impl<V, C, Schedule> HorizonMatch<C, C> for CordialDagView<'_, V, C, Schedule>
where
    Schedule: FollowingBoundary<u64>,
    V: Clone + Eq + Hash + Ord,
    C: Clone + Eq + Hash + Ord,
{
    fn match_horizon(&self, query: &C) -> Option<C> {
        let observation = JustificationRelation::new(self.history);
        let approval = UncontestedRelation::new(self.history, observation);
        let ratification = QuorumOf::new(PreviousRoundWitnesses::new(self.history), approval, TwoThirdsQuorum);
        ObservedHorizonMatchEvidence::new(self.history, self.schedule, self.observer, ratification).match_horizon(query)
    }
}

impl<V, C, Schedule> RatifiedPredecessor for CordialDagView<'_, V, C, Schedule>
where
    Schedule: LeaderSelectionPolicy<u64, V>,
    V: Clone + Eq + Hash + Ord,
    C: Clone + Eq + Hash + Ord,
{
    fn previous_ratified(&self, anchor: &C) -> Option<C> {
        let anchor_slot = self.slot_of(anchor);
        let observation = JustificationRelation::new(self.history);
        let approval = UncontestedRelation::new(self.history, observation);
        let ratification = QuorumOf::new(PreviousRoundWitnesses::new(self.history), approval, TwoThirdsQuorum);
        self.slots()
            .into_iter()
            .rev()
            .filter(|slot| slot < &anchor_slot)
            .filter_map(|slot| self.cordial_leader(&slot))
            .find(|candidate| ratification.relates(anchor, candidate))
    }
}

impl<V, C, Schedule> CordialBlocklace for CordialDagView<'_, V, C, Schedule>
where
    V: Clone + Eq + Ord,
    C: Clone + Eq + Hash + Ord,
{
    fn blocks(&self) -> Vec<C> {
        self.slots().into_iter().flat_map(|slot| self.candidates(&slot)).collect()
    }

    fn observes(&self, viewer: &C, block: &C) -> bool {
        self.history.justification_observes(viewer, block)
    }

    fn approves(&self, viewer: &C, block: &C) -> bool {
        let observation = JustificationRelation::new(self.history);
        UncontestedRelation::new(self.history, observation).relates(viewer, block)
    }
}

/// Collapses Cordial wave leaders from one observer's reachable message history.
#[derive(Debug, Clone, Copy, New)]
pub struct CordialCollapse<Schedule> {
    schedule: Schedule,
}

impl<V, C, Schedule> HistoryCollapse<CommunicationHistory<V, C>, ObservedSlot<V, C>, Closed<C>>
    for CordialCollapse<Schedule>
where
    Schedule: FollowingBoundary<u64> + LeaderSelectionPolicy<u64, V>,
    V: Clone + Eq + Hash + Ord,
    C: Clone + Eq + Hash + Ord,
{
    fn collapse_history(&self, history: &CommunicationHistory<V, C>, query: &ObservedSlot<V, C>) -> Option<Closed<C>> {
        let committee = history.committee_order();
        let selected = matches!(
            self.schedule.select_leaders(&query.slot.round, &committee),
            LeaderSelection::Selected(leaders) if leaders.contains(&query.slot.validator)
        );
        if !selected {
            return None;
        }

        let view = CordialDagView::new(history, &self.schedule, &query.observer);
        match view.cordial_decision(&query.slot) {
            Decision::Commit(candidate) => Some(Closed::Commit(candidate)),
            Decision::Skip(_) => Some(Closed::Skip),
            Decision::Undecided(_) => None,
        }
    }
}

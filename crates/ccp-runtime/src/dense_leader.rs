//! Interprets scheduled leader collapse from shared communication legitimacy.
use crate::CommunicationHistory;
use ccp_communication::{JustificationRelation, MessageCarrier, MessageSlot, ObservedRelationEvidence};
use ccp_core::CandidateSorts;
use ccp_core::{CandidateSpace, Closed, Decision, HistoryCollapse};
use ccp_dense_leader::{DenseLeaderEvidenceCollapseExt, SlotLeader};
use ccp_leader::{AllLeaderSlots, LeaderSelection, LeaderSelectionPolicy};
use ccp_quorum::CommitteeOrder;
use derive_new::new as New;
use std::hash::Hash;

/// Presents a communication history with the leader slots selected by `schedule`.
#[derive(Debug, Clone, Copy, New)]
pub struct ScheduledLeaderView<'a, V, C, Schedule> {
    history: &'a CommunicationHistory<V, C>,
    schedule: &'a Schedule,
}

impl<V, C, Schedule> CandidateSorts for ScheduledLeaderView<'_, V, C, Schedule> {
    type Slot = MessageSlot<V>;
    type Candidate = C;
}

impl<V, C, Schedule> CandidateSpace for ScheduledLeaderView<'_, V, C, Schedule>
where
    V: Clone + Eq + Ord,
    C: Clone + Eq,
{
    fn slots(&self) -> Vec<MessageSlot<V>> {
        self.history.slots()
    }

    fn candidates(&self, slot: &MessageSlot<V>) -> Vec<C> {
        self.history.candidates(slot)
    }

    fn slot_of(&self, candidate: &C) -> MessageSlot<V> {
        self.history.slot_of(candidate)
    }
}

impl<V, C, Schedule> SlotLeader for ScheduledLeaderView<'_, V, C, Schedule>
where
    Schedule: LeaderSelectionPolicy<u64, V>,
    V: Clone + Eq + Ord,
    C: Clone,
{
    fn slot_leader(&self, slot: &MessageSlot<V>) -> Option<C> {
        let committee = self.history.committee_order();
        matches!(
            self.schedule.select_leaders(&slot.round, &committee),
            LeaderSelection::Selected(leaders) if leaders.contains(&slot.validator)
        )
        .then(|| self.history.message_at(slot))
        .flatten()
    }
}

/// Collapses scheduled leader slots from shared legitimacy visible to one observing message.
#[derive(Debug, Clone, Copy, New, PartialEq, Eq)]
pub struct ScheduledLeaderCollapse<Schedule> {
    schedule: Schedule,
}

/// Collapses every occupied slot as a dense leader opportunity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DenseLeaderCollapse;

/// Selects a candidate slot as observed from one message's causal history.
#[derive(Debug, Clone, New, PartialEq, Eq)]
pub struct ObservedSlot<V, C> {
    /// Identifies the candidate slot being classified.
    pub slot: MessageSlot<V>,
    /// Identifies the message that supplies the observation point.
    pub observer: C,
}

impl<V, C, Schedule> HistoryCollapse<CommunicationHistory<V, C>, ObservedSlot<V, C>, Closed<C>>
    for ScheduledLeaderCollapse<Schedule>
where
    Schedule: LeaderSelectionPolicy<u64, V>,
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

        let view = ScheduledLeaderView { history, schedule: &self.schedule };
        let relation = JustificationRelation::new(history);
        let evidence = ObservedRelationEvidence::new(history, &query.observer, relation);
        match view.dense_leader_history_decision(&evidence, &query.slot) {
            Decision::Commit(candidate) => Some(Closed::Commit(candidate)),
            Decision::Skip(_) => Some(Closed::Skip),
            Decision::Undecided(_) => None,
        }
    }
}

impl<V, C> HistoryCollapse<CommunicationHistory<V, C>, ObservedSlot<V, C>, Closed<C>> for DenseLeaderCollapse
where
    V: Clone + Eq + Hash + Ord,
    C: Clone + Eq + Hash + Ord,
{
    fn collapse_history(&self, history: &CommunicationHistory<V, C>, query: &ObservedSlot<V, C>) -> Option<Closed<C>> {
        ScheduledLeaderCollapse::new(AllLeaderSlots).collapse_history(history, query)
    }
}

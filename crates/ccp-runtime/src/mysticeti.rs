//! Interprets Mysticeti collapse directly from a reachable communication history.
use crate::CommunicationHistory;
use ccp_communication::{MessageCarrier, MessageJustification, MessageSlot};
use ccp_core::CandidateSorts;
use ccp_core::{CandidateSpace, Closed, Decision, FollowingBoundary, HistoryCollapse};
use ccp_leader::{LeaderSelection, LeaderSelectionPolicy};
use ccp_mysticeti::{
    CertifiedLink, MysticetiCertificateExt, MysticetiCollapseViewExt, MysticetiDagEvidence, RepairAnchorSlots,
};
use ccp_quorum::{CommitteeOrder, ValidatorSet};
use derive_new::new as New;
use std::hash::Hash;

/// Presents a communication history with Mysticeti's leader schedule and repair anchors.
#[derive(Debug, Clone, Copy, New)]
pub struct ScheduledHistory<'a, Carrier, Schedule> {
    history: &'a Carrier,
    schedule: &'a Schedule,
}

impl<Carrier, Schedule> CandidateSorts for ScheduledHistory<'_, Carrier, Schedule>
where
    Carrier: CandidateSorts,
{
    type Slot = Carrier::Slot;
    type Candidate = Carrier::Candidate;
}

impl<Carrier, Schedule> CandidateSpace for ScheduledHistory<'_, Carrier, Schedule>
where
    Carrier: CandidateSpace,
{
    fn slots(&self) -> Vec<Self::Slot> {
        self.history.slots()
    }

    fn candidates(&self, slot: &Self::Slot) -> Vec<Self::Candidate> {
        self.history.candidates(slot)
    }

    fn slot_of(&self, candidate: &Self::Candidate) -> Self::Slot {
        self.history.slot_of(candidate)
    }
}

impl<Carrier, V, C, Schedule> RepairAnchorSlots for ScheduledHistory<'_, Carrier, Schedule>
where
    Carrier: CandidateSpace<Slot = MessageSlot<V>, Candidate = C>
        + CommitteeOrder<Validator = V>
        + MessageCarrier<Message = C>,
    Schedule: FollowingBoundary<u64> + LeaderSelectionPolicy<u64, V>,
    V: Clone + Eq + Ord,
    C: Clone,
{
    fn repair_anchor_slots(&self, slot: &MessageSlot<V>) -> Vec<MessageSlot<V>> {
        let boundary = self.schedule.following_boundary(&slot.round);
        let committee = self.history.committee_order();
        self.history
            .message_slots()
            .into_iter()
            .filter(|candidate| candidate.round > boundary)
            .filter(|candidate| {
                matches!(
                    self.schedule.select_leaders(&candidate.round, &committee),
                    LeaderSelection::Selected(leaders) if leaders.contains(&candidate.validator)
                )
            })
            .collect()
    }
}

impl<Carrier, V, C, Schedule> CertifiedLink for ScheduledHistory<'_, Carrier, Schedule>
where
    Carrier: CandidateSpace<Slot = MessageSlot<V>, Candidate = C>
        + MessageCarrier<Validator = V, Message = C>
        + MessageJustification
        + ValidatorSet,
    Schedule: FollowingBoundary<u64>,
    V: Clone + Eq + Hash + Ord,
    C: Clone + Eq + Hash + Ord,
{
    fn certified_link(&self, anchor: &C, candidate: &C) -> bool {
        self.history.mysticeti_certified_link(anchor, candidate, self.schedule)
    }
}

/// Collapses one reachable history with Mysticeti's schedule and evidence interpretation.
#[derive(Debug, Clone, Copy, New)]
pub struct MysticetiCollapse<Schedule> {
    schedule: Schedule,
}

impl<V, C, Schedule> HistoryCollapse<CommunicationHistory<V, C>, MessageSlot<V>, Closed<C>>
    for MysticetiCollapse<Schedule>
where
    Schedule: FollowingBoundary<u64> + LeaderSelectionPolicy<u64, V>,
    V: Clone + Eq + Hash + Ord,
    C: Clone + Eq + Hash + Ord,
{
    fn collapse_history(&self, history: &CommunicationHistory<V, C>, slot: &MessageSlot<V>) -> Option<Closed<C>> {
        let committee = history.committee_order();
        let selected = matches!(
            self.schedule.select_leaders(&slot.round, &committee),
            LeaderSelection::Selected(leaders) if leaders.contains(&slot.validator)
        );
        if !selected {
            return None;
        }

        let view = ScheduledHistory { history, schedule: &self.schedule };
        let evidence = MysticetiDagEvidence::new(history, &self.schedule);
        match view.mysticeti_decision(&evidence, slot) {
            Decision::Commit(candidate) => Some(Closed::Commit(candidate)),
            Decision::Skip(_) => Some(Closed::Skip),
            Decision::Undecided(_) => None,
        }
    }
}

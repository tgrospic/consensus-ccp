//! Implements an immutable message history and its lawful input interpreter.
use ccp_communication::CommunicationSorts;
use ccp_communication::{
    AdvancePolicy, ExpansionError, MessageAdmissionExt, MessageCarrier, MessageCarrierExt, MessageInput,
    MessageJustification, MessageSlot,
};
use ccp_core::CandidateSorts;
use ccp_core::{CandidateSpace, HistoryExpansion};
use ccp_leader::CandidateAuthor;
use ccp_quorum::ValidatorSorts;
use ccp_quorum::{CommitteeOrder, QuorumThreshold, ValidatorSet};
use derive_new::new as New;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

/// Retains one reachable round-by-validator communication history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommunicationHistory<V, C> {
    committee: Vec<V>,
    messages: BTreeMap<MessageSlot<V>, Vec<C>>,
    justifications: BTreeMap<C, BTreeSet<C>>,
}

impl<V, C> CommunicationHistory<V, C>
where
    V: Clone + Eq + Hash,
{
    /// Creates the genesis carrier boundary with an explicit validator order.
    ///
    /// # Errors
    ///
    /// Rejects an empty committee or a committee containing a duplicate validator.
    pub fn genesis(committee: Vec<V>) -> Result<Self, ExpansionError> {
        if committee.is_empty() {
            return Err(ExpansionError::EmptyCommittee);
        }
        if committee.iter().collect::<HashSet<_>>().len() != committee.len() {
            return Err(ExpansionError::DuplicateValidator);
        }
        Ok(Self { committee, messages: BTreeMap::new(), justifications: BTreeMap::new() })
    }
}

impl<V, C> CommunicationHistory<V, C> {
    fn insert(&mut self, input: &MessageInput<V, C>)
    where
        V: Clone + Ord,
        C: Clone + Ord,
    {
        self.messages.entry(input.slot.clone()).or_default().push(input.candidate.clone());
        self.justifications.insert(input.candidate.clone(), input.justifications.iter().cloned().collect());
    }
}

impl<V, C> CommunicationSorts for CommunicationHistory<V, C> {
    type Message = C;
}

impl<V, C> MessageCarrier for CommunicationHistory<V, C>
where
    V: Clone + Ord,
    C: Clone,
{
    fn message_slots(&self) -> Vec<MessageSlot<V>> {
        self.messages.keys().cloned().collect()
    }

    fn message_at(&self, slot: &MessageSlot<V>) -> Option<C> {
        self.messages.get(slot).and_then(|messages| messages.first().cloned())
    }

    fn equivocations(&self, slot: &MessageSlot<V>) -> Vec<C> {
        self.messages.get(slot).map(|messages| messages.iter().skip(1).cloned().collect()).unwrap_or_default()
    }
}

impl<V, C> MessageJustification for CommunicationHistory<V, C>
where
    C: Clone + Ord,
{
    fn message_justifications(&self, message: &C) -> Vec<C> {
        self.justifications
            .get(message)
            .map(|justifications| justifications.iter().cloned().collect())
            .unwrap_or_default()
    }
}

impl<V, C> ValidatorSorts for CommunicationHistory<V, C> {
    type Validator = V;
}

impl<V, C> ValidatorSet for CommunicationHistory<V, C>
where
    V: Clone,
{
    fn validators(&self) -> Vec<V> {
        self.committee.clone()
    }
}

impl<V, C> CommitteeOrder for CommunicationHistory<V, C>
where
    V: Clone,
{
    fn committee_order(&self) -> Vec<V> {
        self.committee.clone()
    }
}

impl<V, C> CandidateSorts for CommunicationHistory<V, C> {
    type Slot = MessageSlot<V>;
    type Candidate = C;
}

impl<V, C> CandidateSpace for CommunicationHistory<V, C>
where
    V: Clone + Eq + Ord,
    C: Clone + Eq,
{
    fn slots(&self) -> Vec<MessageSlot<V>> {
        self.message_slots()
    }

    fn candidates(&self, slot: &MessageSlot<V>) -> Vec<C> {
        self.slot_messages(slot)
    }

    fn slot_of(&self, candidate: &C) -> MessageSlot<V> {
        self.message_slot_of(candidate).expect("reachable candidate has exactly one communication slot")
    }
}

impl<V, C> CandidateAuthor for CommunicationHistory<V, C>
where
    V: Clone + Eq + Ord,
    C: Clone + Eq,
{
    fn candidate_author(&self, candidate: &C) -> Option<V> {
        self.message_slot_of(candidate).map(|slot| slot.validator)
    }
}

/// Interprets message inputs according to a round-advance policy.
#[derive(Debug, Clone, Copy, New)]
pub struct CommunicationExpansion<Advance> {
    advance: Advance,
}

impl<V, C, Advance> HistoryExpansion<CommunicationHistory<V, C>> for CommunicationExpansion<Advance>
where
    Advance: AdvancePolicy + QuorumThreshold,
    V: Clone + Eq + Hash + Ord + Debug,
    C: Clone + Eq + Hash + Ord + Debug,
{
    type Input = MessageInput<V, C>;
    type Error = ExpansionError;

    fn expand(
        &self,
        history: &CommunicationHistory<V, C>,
        input: &MessageInput<V, C>,
    ) -> Result<CommunicationHistory<V, C>, ExpansionError> {
        history.admit_message(&self.advance, input)?;
        let mut later = history.clone();
        later.insert(input);
        Ok(later)
    }
}

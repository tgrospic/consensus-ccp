//! Defines the typed message input and the admission rule of communication expansion.
use crate::{AdvancePolicy, AdvancePolicyExt, MessageCarrier, MessageCarrierExt, MessageSlot};
use alux_ext::ext;
use ccp_quorum::{QuorumThreshold, ValidatorSet};
use derive_new::new as New;
use std::collections::HashSet;
use std::hash::Hash;

/// Describes one message offered to communication expansion.
#[derive(Debug, Clone, New, PartialEq, Eq)]
pub struct MessageInput<V, C> {
    /// Identifies the message's communication position.
    pub slot: MessageSlot<V>,
    /// Identifies the candidate carried by the message.
    pub candidate: C,
    /// Identifies the earlier messages directly justified by this message.
    pub justifications: Vec<C>,
}

/// Describes why a genesis committee or a message input is not a lawful communication expansion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpansionError {
    /// Rejects a genesis committee with no validators.
    EmptyCommittee,
    /// Rejects a validator repeated in the genesis committee order.
    DuplicateValidator,
    /// Rejects a message whose author is outside the committee.
    UnknownValidator,
    /// Rejects reuse of an existing candidate identity.
    DuplicateCandidate,
    /// Rejects a justification absent from the current history.
    UnknownJustification,
    /// Rejects a justification that does not precede the proposed message.
    NonCausalJustification,
    /// Rejects a message without the advance policy's required justification quorum.
    InsufficientAdvanceEvidence,
    /// Rejects a round whose equivocators exceed the policy's Byzantine bound.
    EquivocationBoundExceeded,
}

/// Derives which message inputs lawfully expand a communication history.
#[ext(name = MessageAdmissionExt)]
pub impl<This, V, C> This
where
    This: MessageCarrier<Validator = V, Message = C> + ValidatorSet,
    V: Clone + Eq + Hash,
    C: Clone + Eq,
{
    /// Admits `input` as a successor message under `advance`, or returns the first rule it violates.
    ///
    /// # Errors
    ///
    /// Returns the rejection for an unknown author, a reused candidate, an unknown or non-causal
    /// justification, missing advance quorum, or an exceeded equivocation bound.
    fn admit_message<Policy>(&self, advance: &Policy, input: &MessageInput<V, C>) -> Result<(), ExpansionError>
    where
        Policy: AdvancePolicy + QuorumThreshold,
    {
        let validators = self.validators();
        let messages = self.messages();
        if !validators.contains(&input.slot.validator) {
            return Err(ExpansionError::UnknownValidator);
        }
        if messages.contains(&input.candidate) {
            return Err(ExpansionError::DuplicateCandidate);
        }
        if input.justifications.iter().any(|target| !messages.contains(target)) {
            return Err(ExpansionError::UnknownJustification);
        }
        if input
            .justifications
            .iter()
            .filter_map(|target| self.message_slot_of(target))
            .any(|target| target.round >= input.slot.round)
        {
            return Err(ExpansionError::NonCausalJustification);
        }
        if !advance.justifications_advance(self, input.slot.round, &input.justifications) {
            return Err(ExpansionError::InsufficientAdvanceEvidence);
        }

        let mut equivocators = self
            .message_slots()
            .into_iter()
            .filter(|slot| slot.round == input.slot.round && !self.equivocations(slot).is_empty())
            .map(|slot| slot.validator)
            .collect::<HashSet<_>>();
        if self.message_at(&input.slot).is_some() {
            equivocators.insert(input.slot.validator.clone());
        }
        if equivocators.len() > advance.fault_bound(validators.len()) {
            return Err(ExpansionError::EquivocationBoundExceeded);
        }
        Ok(())
    }
}

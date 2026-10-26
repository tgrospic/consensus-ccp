//! Defines communication-carrier slots that identify a round and a validator.
use crate::CommunicationSorts;
use alux_ext::ext;
use ccp_core::{EvidenceRelation, FollowingBoundary, HorizonMatch, SlotNumber};
use ccp_validator::{CommitteeOrder, ValidatorSet, ValidatorSorts, ValidatorWeight};
use ccp_quorum::{CandidateLegitimacyExt, CandidateSupport, ValidatorEvidence};
use derive_more::Display;
use derive_new::new as New;
use std::collections::HashSet;
use std::hash::Hash;
use std::marker::PhantomData;

/// Identifies one validator message position in a round.
#[derive(Debug, Clone, Copy, Display, New, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[display("{round}:{validator}")]
pub struct MessageSlot<V> {
    /// Communication round or height.
    pub round: u64,
    /// Validator identity that produced the message.
    pub validator: V,
}

impl<V> SlotNumber for MessageSlot<V> {
    fn slot_number(&self) -> u64 {
        self.round
    }
}

/// Exposes validator-indexed messages inside a communication carrier.
pub trait MessageCarrier: CommunicationSorts {
    /// Returns every message slot in the carrier, oldest first.
    fn message_slots(&self) -> Vec<MessageSlot<Self::Validator>>;

    /// Returns the canonical message stored in `slot`, if any.
    fn message_at(&self, slot: &MessageSlot<Self::Validator>) -> Option<Self::Message>;

    /// Returns any additional equivocations stored in `slot`.
    fn equivocations(&self, slot: &MessageSlot<Self::Validator>) -> Vec<Self::Message>;
}

/// Derives message-set and slot queries from a communication carrier.
#[ext(name = MessageCarrierExt)]
pub impl<This, V, C> This
where
    This: MessageCarrier<Validator = V, Message = C>,
    V: Clone,
    C: Clone + Eq,
{
    /// Returns the canonical message followed by every retained equivocation in `slot`.
    fn slot_messages(&self, slot: &MessageSlot<V>) -> Vec<C> {
        self.message_at(slot).into_iter().chain(self.equivocations(slot)).collect()
    }

    /// Returns every retained message in slot order.
    fn messages(&self) -> Vec<C> {
        self.message_slots().into_iter().flat_map(|slot| self.slot_messages(&slot)).collect()
    }

    /// Returns the unique slot containing `message`.
    fn message_slot_of(&self, message: &C) -> Option<MessageSlot<V>> {
        let mut slots = self.message_slots().into_iter().filter(|slot| self.slot_messages(slot).contains(message));
        slots.next().filter(|_| slots.next().is_none())
    }

    /// Reports whether `later` preserves every message and slot admission order in this carrier.
    fn message_refines<Later>(&self, later: &Later) -> bool
    where
        Later: MessageCarrier<Validator = V, Message = C>,
        V: Eq,
    {
        self.message_slots().into_iter().all(|slot| {
            let earlier = self.slot_messages(&slot);
            let later = later.message_at(&slot).into_iter().chain(later.equivocations(&slot)).collect::<Vec<_>>();
            later.starts_with(&earlier)
        })
    }
}

/// Exposes direct justification links carried by communication messages.
pub trait MessageJustification: CommunicationSorts {
    /// Returns the messages directly justified by `message`.
    fn message_justifications(&self, message: &Self::Message) -> Vec<Self::Message>;
}

/// Derives the reflexive, transitive justification relation.
#[ext(name = MessageJustificationExt)]
pub impl<This, C> This
where
    This: MessageJustification<Message = C>,
    C: Clone + Eq + Hash,
{
    /// Reports whether `message` transitively justifies `target`.
    fn justification_observes(&self, message: &C, target: &C) -> bool {
        let mut pending = vec![message.clone()];
        let mut visited = HashSet::new();

        while let Some(candidate) = pending.pop() {
            if &candidate == target {
                return true;
            }
            if visited.insert(candidate.clone()) {
                pending.extend(self.message_justifications(&candidate));
            }
        }

        false
    }

    /// Reports whether `later` preserves every direct justification in this carrier.
    fn justification_refines<Later>(&self, later: &Later, messages: &[C]) -> bool
    where
        Later: MessageJustification<Message = C>,
    {
        messages.iter().all(|message| {
            let later = later.message_justifications(message);
            self.message_justifications(message).into_iter().all(|target| later.contains(&target))
        })
    }
}

/// Projects a relation from every retained validator witness.
#[derive(Debug, Clone, Copy, New)]
pub struct RelationEvidence<'a, Carrier, Relation> {
    carrier: &'a Carrier,
    relation: Relation,
}

/// Projects a relation from the direct communication cut of one observing message.
#[derive(Debug, Clone, Copy, New)]
pub struct ObservedRelationEvidence<'a, Carrier, Relation, C> {
    carrier: &'a Carrier,
    observer: &'a C,
    relation: Relation,
}

/// Projects a relation from witnesses admitted by a fixed evidence boundary.
#[derive(Debug, Clone, Copy, New)]
pub struct HorizonMatchEvidence<'a, Carrier, Boundary, Relation, C> {
    carrier: &'a Carrier,
    boundary: &'a Boundary,
    relation: Relation,
    #[new(default)]
    candidate: PhantomData<C>,
}

/// Projects a boundary relation visible from one observing message.
#[derive(Debug, Clone, Copy, New)]
pub struct ObservedHorizonMatchEvidence<'a, Carrier, Boundary, Relation, C> {
    carrier: &'a Carrier,
    boundary: &'a Boundary,
    observer: &'a C,
    relation: Relation,
}

impl<Carrier, Relation> ValidatorSorts for RelationEvidence<'_, Carrier, Relation>
where
    Carrier: ValidatorSorts,
{
    type Validator = Carrier::Validator;
    type Weight = Carrier::Weight;
}

impl<Carrier, Relation> ValidatorSet for RelationEvidence<'_, Carrier, Relation>
where
    Carrier: ValidatorSet,
{
    fn validators(&self) -> Vec<Self::Validator> {
        self.carrier.validators()
    }
}

impl<Carrier, Relation> ValidatorWeight for RelationEvidence<'_, Carrier, Relation>
where
    Carrier: ValidatorWeight,
{
    fn weight(&self, validator: &Self::Validator) -> Self::Weight {
        self.carrier.weight(validator)
    }
}

impl<Carrier, Relation, V> CommitteeOrder for RelationEvidence<'_, Carrier, Relation>
where
    Carrier: CommitteeOrder<Validator = V>,
{
    fn committee_order(&self) -> Vec<V> {
        self.carrier.committee_order()
    }
}

impl<Carrier, Relation, V, C> ValidatorEvidence<CandidateSupport, C> for RelationEvidence<'_, Carrier, Relation>
where
    Carrier: MessageCarrier<Validator = V, Message = C>,
    Relation: EvidenceRelation<C, C>,
    V: Eq,
    C: Clone + Eq,
{
    fn supports(&self, _kind: &CandidateSupport, validator: &V, target: &C) -> bool {
        self.carrier
            .message_slots()
            .into_iter()
            .filter(|slot| &slot.validator == validator)
            .flat_map(|slot| self.carrier.message_at(&slot).into_iter().chain(self.carrier.equivocations(&slot)))
            .any(|message| self.relation.relates(&message, target))
    }
}

impl<Carrier, Relation, C> ValidatorSorts for ObservedRelationEvidence<'_, Carrier, Relation, C>
where
    Carrier: ValidatorSorts,
{
    type Validator = Carrier::Validator;
    type Weight = Carrier::Weight;
}

impl<Carrier, Relation, C> ValidatorSet for ObservedRelationEvidence<'_, Carrier, Relation, C>
where
    Carrier: ValidatorSet,
{
    fn validators(&self) -> Vec<Self::Validator> {
        self.carrier.validators()
    }
}

impl<Carrier, Relation, C> ValidatorWeight for ObservedRelationEvidence<'_, Carrier, Relation, C>
where
    Carrier: ValidatorWeight,
{
    fn weight(&self, validator: &Self::Validator) -> Self::Weight {
        self.carrier.weight(validator)
    }
}

impl<Carrier, Relation, V, C> ValidatorEvidence<CandidateSupport, C>
    for ObservedRelationEvidence<'_, Carrier, Relation, C>
where
    Carrier:
        MessageCarrier<Validator = V, Message = C> + MessageJustification<Message = C> + ValidatorSet<Validator = V>,
    Relation: EvidenceRelation<C, C>,
    V: Clone + Eq,
    C: Clone + Eq + Hash,
{
    fn supports(&self, _kind: &CandidateSupport, validator: &V, target: &C) -> bool {
        let Some(observer_slot) = self.carrier.message_slot_of(self.observer) else {
            return false;
        };

        self.carrier
            .message_justifications(self.observer)
            .into_iter()
            .filter_map(|message| self.carrier.message_slot_of(&message).map(|slot| (message, slot)))
            .any(|(message, slot)| {
                &slot.validator == validator
                    && slot.round.saturating_add(1) == observer_slot.round
                    && self.relation.relates(&message, target)
            })
    }
}

impl<Carrier, Boundary, Relation, C> ValidatorSorts for HorizonMatchEvidence<'_, Carrier, Boundary, Relation, C>
where
    Carrier: ValidatorSorts,
{
    type Validator = Carrier::Validator;
    type Weight = Carrier::Weight;
}

impl<Carrier, Boundary, Relation, C> ValidatorSet for HorizonMatchEvidence<'_, Carrier, Boundary, Relation, C>
where
    Carrier: ValidatorSet,
{
    fn validators(&self) -> Vec<Self::Validator> {
        self.carrier.validators()
    }
}

impl<Carrier, Boundary, Relation, C> ValidatorWeight for HorizonMatchEvidence<'_, Carrier, Boundary, Relation, C>
where
    Carrier: ValidatorWeight,
{
    fn weight(&self, validator: &Self::Validator) -> Self::Weight {
        self.carrier.weight(validator)
    }
}

impl<Carrier, Boundary, Relation, C> ValidatorSorts for ObservedHorizonMatchEvidence<'_, Carrier, Boundary, Relation, C>
where
    Carrier: ValidatorSorts,
{
    type Validator = Carrier::Validator;
    type Weight = Carrier::Weight;
}

impl<Carrier, Boundary, Relation, C> ValidatorSet for ObservedHorizonMatchEvidence<'_, Carrier, Boundary, Relation, C>
where
    Carrier: ValidatorSet,
{
    fn validators(&self) -> Vec<Self::Validator> {
        self.carrier.validators()
    }
}

impl<Carrier, Boundary, Relation, C> ValidatorWeight for ObservedHorizonMatchEvidence<'_, Carrier, Boundary, Relation, C>
where
    Carrier: ValidatorWeight,
{
    fn weight(&self, validator: &Self::Validator) -> Self::Weight {
        self.carrier.weight(validator)
    }
}

impl<Carrier, Boundary, Relation, V, C> ValidatorEvidence<CandidateSupport, C>
    for HorizonMatchEvidence<'_, Carrier, Boundary, Relation, C>
where
    Carrier: MessageCarrier<Validator = V, Message = C> + ValidatorSet,
    Boundary: FollowingBoundary<u64>,
    Relation: EvidenceRelation<C, C>,
    V: Clone + Eq,
    C: Clone + Eq,
{
    fn supports(&self, _kind: &CandidateSupport, validator: &V, target: &C) -> bool {
        let Some(target_slot) = self.carrier.message_slot_of(target) else {
            return false;
        };
        let evidence_round = self.boundary.following_boundary(&target_slot.round);
        self.carrier
            .message_slots()
            .into_iter()
            .filter(|slot| slot.round == evidence_round && &slot.validator == validator)
            .flat_map(|slot| self.carrier.slot_messages(&slot))
            .any(|message| self.relation.relates(&message, target))
    }
}

impl<Carrier, Boundary, Relation, V, C> ValidatorEvidence<CandidateSupport, C>
    for ObservedHorizonMatchEvidence<'_, Carrier, Boundary, Relation, C>
where
    Carrier:
        MessageCarrier<Validator = V, Message = C> + MessageJustification<Message = C> + ValidatorSet<Validator = V>,
    Boundary: FollowingBoundary<u64>,
    Relation: EvidenceRelation<C, C>,
    V: Clone + Eq,
    C: Clone + Eq + Hash,
{
    fn supports(&self, _kind: &CandidateSupport, validator: &V, target: &C) -> bool {
        let Some(target_slot) = self.carrier.message_slot_of(target) else {
            return false;
        };
        let evidence_round = self.boundary.following_boundary(&target_slot.round);
        self.carrier
            .message_slots()
            .into_iter()
            .filter(|slot| slot.round == evidence_round && &slot.validator == validator)
            .flat_map(|slot| self.carrier.slot_messages(&slot))
            .filter(|message| self.carrier.justification_observes(self.observer, message))
            .any(|message| self.relation.relates(&message, target))
    }
}

impl<Carrier, Boundary, Relation, V, W, C> HorizonMatch<C, C> for HorizonMatchEvidence<'_, Carrier, Boundary, Relation, C>
where
    Carrier: MessageCarrier<Validator = V, Message = C> + ValidatorSet + ValidatorWeight<Weight = W>,
    Boundary: FollowingBoundary<u64>,
    Relation: EvidenceRelation<C, C>,
    V: Clone + Eq + Hash,
    W: Into<u128>,
    C: Clone + Eq,
{
    fn match_horizon(&self, query: &C) -> Option<C> {
        self.two_thirds_legitimate(query).then(|| query.clone())
    }
}

impl<Carrier, Boundary, Relation, V, W, C> HorizonMatch<C, C>
    for ObservedHorizonMatchEvidence<'_, Carrier, Boundary, Relation, C>
where
    Carrier:
        MessageCarrier<Validator = V, Message = C> + MessageJustification<Message = C> + ValidatorSet<Validator = V>
            + ValidatorWeight<Weight = W>,
    Boundary: FollowingBoundary<u64>,
    Relation: EvidenceRelation<C, C>,
    V: Clone + Eq + Hash,
    W: Into<u128>,
    C: Clone + Eq + Hash,
{
    fn match_horizon(&self, query: &C) -> Option<C> {
        self.two_thirds_legitimate(query).then(|| query.clone())
    }
}

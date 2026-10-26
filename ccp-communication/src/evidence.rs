use crate::{MessageCarrier, MessageCarrierExt, MessageJustification, MessageJustificationExt};
use ccp_core::EvidenceRelation;
use ccp_quorum::WitnessSelection;
use ccp_validator::{ValidatorSet, ValidatorSorts, ValidatorWeight};
use derive_new::new as New;
use std::{hash::Hash, marker::PhantomData};

/// Interprets transitive justification as the base communication relation.
#[derive(Debug, Clone, Copy, New)]
pub struct JustificationRelation<'a, Carrier> {
    carrier: &'a Carrier,
}

impl<Carrier, C> EvidenceRelation<C, C> for JustificationRelation<'_, Carrier>
where
    Carrier: MessageJustification<Message = C>,
    C: Clone + Eq + Hash,
{
    fn relates(&self, witness: &C, target: &C) -> bool {
        self.carrier.justification_observes(witness, target)
    }
}

/// Excludes a target when the witness also observes a conflicting candidate in its slot.
#[derive(Debug, Clone, Copy, New)]
pub struct UncontestedRelation<'a, Carrier, Relation, V> {
    carrier: &'a Carrier,
    relation: Relation,
    #[new(default)]
    validator: PhantomData<V>,
}

impl<Carrier, Relation, V, C> EvidenceRelation<C, C> for UncontestedRelation<'_, Carrier, Relation, V>
where
    Carrier: MessageCarrier<Validator = V, Message = C>,
    Relation: EvidenceRelation<C, C>,
    V: Clone,
    C: Clone + Eq,
{
    fn relates(&self, witness: &C, target: &C) -> bool {
        let Some(slot) = self.carrier.message_slot_of(target) else {
            return false;
        };
        self.relation.relates(witness, target)
            && self
                .carrier
                .slot_messages(&slot)
                .into_iter()
                .filter(|candidate| candidate != target)
                .all(|conflict| !self.relation.relates(witness, &conflict))
    }
}

/// Selects direct justifications from the immediately preceding round.
#[derive(Debug, Clone, Copy, New)]
pub struct PreviousRoundWitnesses<'a, Carrier> {
    carrier: &'a Carrier,
}

impl<Carrier, V, C> WitnessSelection<C> for PreviousRoundWitnesses<'_, Carrier>
where
    Carrier:
        MessageCarrier<Validator = V, Message = C> + MessageJustification<Message = C> + ValidatorSet<Validator = V>,
    V: Clone,
    C: Clone + Eq,
{
    type Evidence = C;

    fn witnesses(&self, witness: &C) -> Vec<(V, C)> {
        let Some(witness_slot) = self.carrier.message_slot_of(witness) else {
            return Vec::new();
        };
        self.carrier
            .message_justifications(witness)
            .into_iter()
            .filter_map(|support| self.carrier.message_slot_of(&support).map(|slot| (slot, support)))
            .filter(|(slot, _)| slot.round.checked_add(1) == Some(witness_slot.round))
            .map(|(slot, support)| (slot.validator, support))
            .collect()
    }
}

impl<Carrier> ValidatorSorts for PreviousRoundWitnesses<'_, Carrier>
where
    Carrier: ValidatorSorts,
{
    type Validator = Carrier::Validator;
    type Weight = Carrier::Weight;
}

impl<Carrier> ValidatorSet for PreviousRoundWitnesses<'_, Carrier>
where
    Carrier: ValidatorSet,
{
    fn validators(&self) -> Vec<Self::Validator> {
        self.carrier.validators()
    }
}

impl<Carrier> ValidatorWeight for PreviousRoundWitnesses<'_, Carrier>
where
    Carrier: ValidatorWeight,
{
    fn weight(&self, validator: &Self::Validator) -> Self::Weight {
        self.carrier.weight(validator)
    }
}

use crate::{QuorumThreshold, QuorumThresholdExt, ValidatorSet, ValidatorSorts};
use alux_ext::ext;
use ccp_core::EvidenceRelation;
use derive_new::new as New;
use std::{collections::HashSet, hash::Hash};

/// Selects attributed evidence visible to one witness.
///
/// Selection determines the evidence scope, independently of the supported proposition and
/// quorum threshold. Repeated evidence from one validator contributes at most one quorum vote.
pub trait WitnessSelection<W>: ValidatorSorts {
    /// Identifies one selected piece of evidence.
    type Evidence;

    /// Returns the attributed evidence available to `witness` in this selection.
    fn witnesses(&self, witness: &W) -> Vec<(Self::Validator, Self::Evidence)>;
}

/// Composes witness selection, a support relation, and a quorum threshold.
#[derive(Debug, Clone, Copy, New)]
pub struct QuorumOf<Selection, Relation, Threshold> {
    selection: Selection,
    relation: Relation,
    threshold: Threshold,
}

/// Derives quorum support without choosing a temporal scope or threshold.
#[ext(name = WitnessQuorumExt)]
pub impl<This, W, V, E> This
where
    This: WitnessSelection<W, Validator = V, Evidence = E> + ValidatorSet,
    V: Eq + Hash,
{
    /// Counts distinct committee members whose selected evidence supports `target`.
    fn quorum_supporters<Relation, T>(&self, relation: &Relation, witness: &W, target: &T) -> HashSet<V>
    where
        Relation: EvidenceRelation<E, T>,
    {
        let validators = self.validators();
        self.witnesses(witness)
            .into_iter()
            .filter(|(validator, evidence)| validators.contains(validator) && relation.relates(evidence, target))
            .map(|(validator, _)| validator)
            .collect()
    }

    /// Reports whether selected support meets the explicitly supplied threshold.
    fn witness_quorum<Relation, Threshold, T>(
        &self,
        relation: &Relation,
        threshold: &Threshold,
        witness: &W,
        target: &T,
    ) -> bool
    where
        Relation: EvidenceRelation<E, T>,
        Threshold: QuorumThreshold,
    {
        threshold.meets_quorum(self.quorum_supporters(relation, witness, target).len(), self.validators().len())
    }
}

impl<Selection, Relation, Threshold, W, T, V, E> EvidenceRelation<W, T> for QuorumOf<Selection, Relation, Threshold>
where
    Selection: WitnessSelection<W, Validator = V, Evidence = E> + ValidatorSet,
    Relation: EvidenceRelation<E, T>,
    Threshold: QuorumThreshold,
    V: Eq + Hash,
{
    fn relates(&self, witness: &W, target: &T) -> bool {
        self.selection.witness_quorum(&self.relation, &self.threshold, witness, target)
    }
}

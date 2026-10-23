//! Implements a constructive proof-of-work block-tree carrier.
use ccp_confidence::ConfidenceEvidence;
use ccp_core::CandidateSorts;
use ccp_core::{CandidateSpace, HistoryCollapse, HistoryExpansion};
use ccp_nakamoto::{
    BlockParent, ConfirmationConfidence, NakamotoCollapseExt, NakamotoConfidenceExt, NakamotoConfirmation,
    NakamotoPolicy, NakamotoPrefix, ProofOfWork,
};
use derive_new::new as New;
use std::collections::BTreeMap;
use std::hash::Hash;

/// Describes one externally verified proof-of-work block offered to the carrier.
#[derive(Debug, Clone, New, PartialEq, Eq)]
pub struct WorkBlockInput<C> {
    /// Identifies the new block.
    pub block: C,
    /// Identifies its parent, or no parent for genesis.
    pub parent: Option<C>,
    /// Carries the verified positive work represented by the proof.
    pub work: u128,
}

/// Describes why a proof-of-work block cannot expand the current tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkExpansionError {
    /// Rejects a block identity already present in the tree.
    DuplicateBlock,
    /// Rejects a proof that contributes no positive work.
    InvalidWork,
    /// Rejects a second genesis block or a genesis block after another block.
    InvalidGenesis,
    /// Rejects a block whose parent is absent from the current tree.
    UnknownParent,
}

/// Retains one reachable proof-of-work block tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkHistory<C> {
    heights: BTreeMap<u64, Vec<C>>,
    block_heights: BTreeMap<C, u64>,
    parents: BTreeMap<C, C>,
    work: BTreeMap<C, u128>,
}

impl<C> Default for WorkHistory<C> {
    fn default() -> Self {
        Self {
            heights: BTreeMap::new(),
            block_heights: BTreeMap::new(),
            parents: BTreeMap::new(),
            work: BTreeMap::new(),
        }
    }
}

impl<C> CandidateSorts for WorkHistory<C> {
    type Slot = u64;
    type Candidate = C;
}

impl<C> CandidateSpace for WorkHistory<C>
where
    C: Clone + Ord,
{
    fn slots(&self) -> Vec<u64> {
        self.heights.keys().copied().collect()
    }

    fn candidates(&self, height: &u64) -> Vec<C> {
        self.heights.get(height).cloned().unwrap_or_default()
    }

    fn slot_of(&self, block: &C) -> u64 {
        self.block_heights.get(block).copied().expect("reachable block has one height")
    }
}

impl<C> BlockParent for WorkHistory<C>
where
    C: Clone + Ord,
{
    fn block_parent(&self, block: &C) -> Option<C> {
        self.parents.get(block).cloned()
    }
}

impl<C> ProofOfWork for WorkHistory<C>
where
    C: Clone + Ord,
{
    fn block_work(&self, block: &C) -> Option<u128> {
        self.work.get(block).copied()
    }
}

impl<C> ConfidenceEvidence<NakamotoConfirmation, C> for WorkHistory<C>
where
    C: Clone + Eq + Hash + Ord,
{
    type Confidence = ConfirmationConfidence;

    fn confidence(&self, candidate: &C) -> Option<ConfirmationConfidence> {
        self.nakamoto_confirmation_confidence(candidate)
    }
}

/// Interprets verified blocks as lawful block-tree expansions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WorkExpansion;

impl<C> HistoryExpansion<WorkHistory<C>> for WorkExpansion
where
    C: Clone + Ord,
{
    type Input = WorkBlockInput<C>;
    type Error = WorkExpansionError;

    fn expand(
        &self,
        history: &WorkHistory<C>,
        input: &WorkBlockInput<C>,
    ) -> Result<WorkHistory<C>, WorkExpansionError> {
        if history.block_heights.contains_key(&input.block) {
            return Err(WorkExpansionError::DuplicateBlock);
        }
        if input.work == 0 {
            return Err(WorkExpansionError::InvalidWork);
        }

        let height = match &input.parent {
            None if history.block_heights.is_empty() => 0,
            None => return Err(WorkExpansionError::InvalidGenesis),
            Some(parent) => history
                .block_heights
                .get(parent)
                .copied()
                .map(|height| height.saturating_add(1))
                .ok_or(WorkExpansionError::UnknownParent)?,
        };

        let mut later = history.clone();
        later.heights.entry(height).or_default().push(input.block.clone());
        later.block_heights.insert(input.block.clone(), height);
        if let Some(parent) = &input.parent {
            later.parents.insert(input.block.clone(), parent.clone());
        }
        later.work.insert(input.block.clone(), input.work);
        Ok(later)
    }
}

/// Projects the selected proof-of-work chain through a confirmation-depth policy.
#[derive(Debug, Clone, Copy, New)]
pub struct NakamotoCollapse {
    policy: NakamotoPolicy,
}

impl<C> HistoryCollapse<WorkHistory<C>, (), NakamotoPrefix<C>> for NakamotoCollapse
where
    C: Clone + Eq + Hash + Ord,
{
    fn collapse_history(&self, history: &WorkHistory<C>, _query: &()) -> Option<NakamotoPrefix<C>> {
        history.nakamoto_prefix(&self.policy)
    }
}

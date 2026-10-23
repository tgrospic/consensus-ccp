//! Specifies greatest-work fork choice, prefix collapse, and separate risk interpretation.
use alux_ext::ext;
use ccp_confidence::{AcceptanceThreshold, ConfidenceAcceptanceExt, ConfidenceEvidence};
use ccp_core::{CandidateSorts, CandidateSpace};
use derive_new::new as New;
use std::collections::HashSet;
use std::hash::Hash;

/// Exposes a block's parent justification.
pub trait BlockParent: CandidateSorts {
    /// Returns the parent block, or `None` for a genesis block.
    fn block_parent(&self, block: &Self::Candidate) -> Option<Self::Candidate>;
}

/// Exposes authenticated proof-of-work carried by a block.
pub trait ProofOfWork: CandidateSorts {
    /// Returns the block's positive work when its proof is valid.
    fn block_work(&self, block: &Self::Candidate) -> Option<u128>;
}

/// Exposes the confirmation depth retained as a tentative suffix.
pub trait ConfirmationDepth {
    /// Returns the number of newest blocks excluded from the accepted prefix.
    fn confirmations(&self) -> usize;
}

/// Exposes the adversarial hash-power assumption used by the risk model.
pub trait AdversarialHashFraction {
    /// Returns the numerator of the adversarial hash-power fraction.
    fn adversarial_hash_numerator(&self) -> u64;

    /// Returns the denominator of the adversarial hash-power fraction.
    fn adversarial_hash_denominator(&self) -> u64;
}

/// Identifies confirmation depth as Nakamoto's confidence algebra.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct NakamotoConfirmation;

/// Counts successors above a block on the uniquely selected greatest-work chain.
#[derive(Debug, Clone, Copy, New, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConfirmationConfidence(pub usize);

/// Configures the confirmation depth used by prefix collapse.
#[derive(Debug, Clone, Copy, New, PartialEq, Eq)]
pub struct NakamotoPolicy {
    confirmations: usize,
}

/// Configures the adversarial hash-power assumption used by risk interpretation.
#[derive(Debug, Clone, Copy, New, PartialEq, Eq)]
pub struct NakamotoRiskModel {
    adversarial_hash_numerator: u64,
    adversarial_hash_denominator: u64,
}

impl ConfirmationDepth for NakamotoPolicy {
    fn confirmations(&self) -> usize {
        self.confirmations
    }
}

impl AcceptanceThreshold<NakamotoConfirmation, ConfirmationConfidence> for NakamotoPolicy {
    fn accepts(&self, confidence: &ConfirmationConfidence) -> bool {
        confidence.0 >= self.confirmations
    }
}

impl AdversarialHashFraction for NakamotoRiskModel {
    fn adversarial_hash_numerator(&self) -> u64 {
        self.adversarial_hash_numerator
    }

    fn adversarial_hash_denominator(&self) -> u64 {
        self.adversarial_hash_denominator
    }
}

/// Represents the symbolic catch-up bound `(numerator / denominator) ^ exponent`.
#[derive(Debug, Clone, Copy, New, PartialEq, Eq)]
pub struct ReversalRiskBound {
    /// Adversarial share in the biased random-walk ratio.
    pub numerator: u64,
    /// Honest share in the biased random-walk ratio.
    pub denominator: u64,
    /// Confirmation depth applied to the ratio.
    pub exponent: usize,
}

/// Describes greatest-work fork choice without hiding equal-work ambiguity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NakamotoForkChoice<C> {
    /// Indicates that no valid positive-work chain exists.
    Empty,
    /// Selects the unique greatest-work tip.
    Selected {
        /// Greatest-work chain tip.
        tip: C,
        /// Cumulative work ending at `tip`.
        cumulative_work: u128,
    },
    /// Retains all equal-work tips until later work resolves the fork.
    Tied {
        /// Equal-work tips.
        tips: Vec<C>,
        /// Shared cumulative work of the tied tips.
        cumulative_work: u128,
    },
}

/// Projects a greatest-work chain into accepted and tentative prefixes.
#[derive(Debug, Clone, New, PartialEq, Eq)]
pub struct NakamotoPrefix<C> {
    /// Unique greatest-work tip used by this projection.
    pub tip: C,
    /// Prefix accepted by the configured confirmation-depth policy.
    pub accepted: Vec<C>,
    /// Newest suffix still exposed to reorganization.
    pub tentative: Vec<C>,
}

/// Derives the symbolic biased-random-walk reversal bound.
#[ext(name = NakamotoRiskExt)]
pub impl<This> This
where
    This: AdversarialHashFraction,
{
    /// Returns `1` without an honest majority, otherwise the catch-up bound `(q / (1 - q)) ^ k`.
    fn reversal_risk_bound(&self, deficit: usize) -> ReversalRiskBound {
        let adversarial = self.adversarial_hash_numerator();
        let total = self.adversarial_hash_denominator();
        let honest = total.saturating_sub(adversarial);
        if total == 0 || adversarial >= honest {
            ReversalRiskBound::new(1, 1, 1)
        } else {
            ReversalRiskBound::new(adversarial, honest, deficit)
        }
    }
}

/// Derives valid chains and greatest-work fork choice.
#[ext(name = NakamotoForkChoiceExt)]
pub impl<This, C> This
where
    This: CandidateSpace<Candidate = C> + BlockParent + ProofOfWork,
    C: Clone + Eq + Hash,
{
    /// Returns the valid parent chain ending at `tip`, oldest first.
    fn nakamoto_chain(&self, tip: &C) -> Option<Vec<C>> {
        let known = self.slots().into_iter().flat_map(|slot| self.candidates(&slot)).collect::<HashSet<_>>();
        let mut seen = HashSet::new();
        let mut reversed = Vec::new();
        let mut current = Some(tip.clone());

        while let Some(block) = current {
            if !known.contains(&block)
                || !seen.insert(block.clone())
                || self.block_work(&block).is_none_or(|work| work == 0)
            {
                return None;
            }
            current = self.block_parent(&block);
            reversed.push(block);
        }
        reversed.reverse();
        Some(reversed)
    }

    /// Returns the cumulative work of the valid chain ending at `tip`.
    fn nakamoto_cumulative_work(&self, tip: &C) -> Option<u128> {
        self.nakamoto_chain(tip)?.iter().try_fold(0_u128, |total, block| total.checked_add(self.block_work(block)?))
    }

    /// Selects the unique greatest-work tip or preserves an equal-work tie.
    fn nakamoto_fork_choice(&self) -> NakamotoForkChoice<C> {
        let weighted = self
            .slots()
            .into_iter()
            .flat_map(|slot| self.candidates(&slot))
            .filter_map(|tip| self.nakamoto_cumulative_work(&tip).map(|work| (tip, work)))
            .collect::<Vec<_>>();
        let Some(maximum) = weighted.iter().map(|(_, work)| *work).max() else {
            return NakamotoForkChoice::Empty;
        };
        let tips = weighted.into_iter().filter_map(|(tip, work)| (work == maximum).then_some(tip)).collect::<Vec<_>>();
        tips.first().filter(|_| tips.len() == 1).cloned().map_or_else(
            || NakamotoForkChoice::Tied { tips, cumulative_work: maximum },
            |tip| NakamotoForkChoice::Selected { tip, cumulative_work: maximum },
        )
    }
}

/// Derives confirmation confidence from the uniquely selected greatest-work chain.
#[ext(name = NakamotoConfidenceExt)]
pub impl<This, C> This
where
    This: CandidateSpace<Candidate = C> + BlockParent + ProofOfWork,
    C: Clone + Eq + Hash,
{
    /// Returns the number of selected-chain successors above `block`.
    fn nakamoto_confirmation_confidence(&self, block: &C) -> Option<ConfirmationConfidence> {
        let NakamotoForkChoice::Selected { tip, .. } = self.nakamoto_fork_choice() else {
            return None;
        };
        let chain = self.nakamoto_chain(&tip)?;
        chain
            .iter()
            .position(|candidate| candidate == block)
            .map(|index| ConfirmationConfidence::new(chain.len().saturating_sub(index + 1)))
    }
}

/// Derives confirmation-depth prefix collapse from greatest-work fork choice.
#[ext(name = NakamotoCollapseExt)]
pub impl<This, C> This
where
    This: CandidateSpace<Candidate = C> + BlockParent + ProofOfWork,
    C: Clone + Eq + Hash,
{
    /// Returns an accepted prefix only when greatest-work fork choice is unique.
    fn nakamoto_prefix<Policy>(&self, policy: &Policy) -> Option<NakamotoPrefix<C>>
    where
        This: ConfidenceEvidence<NakamotoConfirmation, C, Confidence = ConfirmationConfidence>,
        Policy: AcceptanceThreshold<NakamotoConfirmation, ConfirmationConfidence>,
    {
        let NakamotoForkChoice::Selected { tip, .. } = self.nakamoto_fork_choice() else {
            return None;
        };
        let chain = self.nakamoto_chain(&tip)?;
        let split = chain
            .iter()
            .position(|block| !policy.accepts_candidate(self, block, NakamotoConfirmation))
            .unwrap_or(chain.len());
        let (accepted, tentative) = chain.split_at(split);
        Some(NakamotoPrefix::new(tip, accepted.to_vec(), tentative.to_vec()))
    }
}

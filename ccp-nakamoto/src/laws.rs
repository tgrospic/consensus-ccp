use crate::{
    AdversarialHashFraction, BlockParent, ConfirmationConfidence, ConfirmationDepth, NakamotoCollapseExt,
    NakamotoConfirmation, NakamotoForkChoice, NakamotoForkChoiceExt, ProofOfWork,
};
use alux_ext::ext;
use anyhow::{Result, bail, ensure};
use ccp_confidence::{AcceptanceThreshold, ConfidenceEvidence};
use ccp_core::CandidateSpace;
use std::{collections::HashSet, fmt::Debug, hash::Hash};

/// Checks Nakamoto block-tree and prefix-collapse obligations.
#[ext(name = NakamotoLawsExt)]
pub impl<This, S, C> This
where
    This: CandidateSpace<Slot = S, Candidate = C>
        + BlockParent
        + ProofOfWork
        + ConfidenceEvidence<NakamotoConfirmation, C, Confidence = ConfirmationConfidence>,
    C: Clone + Eq + Hash,
{
    /// Checks that every valid-work block has a finite, ordered, positive-work parent chain.
    fn check_nakamoto_tree(&self) -> Result<()>
    where
        S: Ord + Debug,
        C: Debug,
    {
        let candidates = self.slots().into_iter().flat_map(|slot| self.candidates(&slot)).collect::<HashSet<_>>();
        candidates.iter().filter(|block| self.block_work(block).is_some()).try_for_each(|block| {
            ensure!(self.block_work(block).is_some_and(|work| work > 0), "PoW block has zero work: {block:?}");
            ensure!(self.nakamoto_chain(block).is_some(), "PoW block has an invalid parent chain: {block:?}");
            if let Some(parent) = self.block_parent(block) {
                ensure!(
                    candidates.contains(&parent),
                    "PoW block references a missing parent: block={block:?}, parent={parent:?}"
                );
                ensure!(
                    self.slot_of(&parent) < self.slot_of(block),
                    "PoW parent does not precede its child: block={block:?}, parent={parent:?}"
                );
            }
            Ok(())
        })
    }

    /// Checks that an accepted prefix partitions the uniquely selected greatest-work chain.
    fn check_nakamoto_prefix<Policy>(&self, policy: &Policy) -> Result<()>
    where
        Policy: ConfirmationDepth + AcceptanceThreshold<NakamotoConfirmation, ConfirmationConfidence>,
        C: Debug,
    {
        match (self.nakamoto_fork_choice(), self.nakamoto_prefix(policy)) {
            (NakamotoForkChoice::Selected { tip, .. }, Some(prefix)) => {
                let chain = self.nakamoto_chain(&tip).expect("selected tip has a valid chain");
                let projected = prefix.accepted.iter().chain(&prefix.tentative).cloned().collect::<Vec<_>>();
                ensure!(prefix.tip == tip, "Nakamoto prefix uses a different tip");
                ensure!(projected == chain, "Nakamoto prefix does not partition the selected chain");
                ensure!(
                    prefix
                        .accepted
                        .iter()
                        .all(|block| self.confidence(block).is_some_and(|confidence| policy.accepts(&confidence))),
                    "Nakamoto accepted prefix contains insufficient confirmation confidence"
                );
                ensure!(
                    prefix
                        .tentative
                        .iter()
                        .all(|block| self.confidence(block).is_none_or(|confidence| !policy.accepts(&confidence))),
                    "Nakamoto tentative suffix contains accepted confirmation confidence"
                );
                ensure!(
                    prefix.tentative.len() == policy.confirmations().min(chain.len()),
                    "Nakamoto tentative suffix has the wrong confirmation depth"
                );
            }
            (NakamotoForkChoice::Empty | NakamotoForkChoice::Tied { .. }, None) => {}
            (choice, prefix) => bail!("Nakamoto fork choice and prefix disagree: choice={choice:?}, prefix={prefix:?}"),
        }
        Ok(())
    }
}

/// Checks Nakamoto risk-model obligations independently from prefix collapse.
#[ext(name = NakamotoRiskLawsExt)]
pub impl<This> This
where
    This: AdversarialHashFraction,
{
    /// Checks that the adversarial hash-power fraction is defined and bounded by one.
    fn check_nakamoto_risk_model(&self) -> Result<()> {
        ensure!(
            self.adversarial_hash_denominator() > 0
                && self.adversarial_hash_numerator() <= self.adversarial_hash_denominator(),
            "invalid adversarial hash-power fraction"
        );
        Ok(())
    }
}

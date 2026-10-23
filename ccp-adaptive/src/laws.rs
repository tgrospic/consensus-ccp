use crate::{AdaptiveEvidence, AdaptiveIndirectExt, AdaptiveLeaderExt, CertifiedPredecessor};
use alux_ext::ext;
use anyhow::{Result, ensure};
use ccp_core::{CandidateRestriction, CandidateSpace, FollowingBoundary, PredecessorChainLawsExt};
use ccp_leader::{CandidateAuthor, LeaderSelectionPolicy};
use ccp_quorum::{CandidateLegitimacyExt, CandidateSupport, CommitteeOrder, ValidatorEvidence, ValidatorSet};
use std::{fmt::Debug, hash::Hash};

/// Checks adaptive anchor-chain obligations.
#[ext(name = AdaptiveLawsExt)]
pub impl<This, S, C> This
where
    This: CandidateSpace<Slot = S, Candidate = C> + CertifiedPredecessor,
{
    /// Checks that an indirect chain stays above the old frontier and increases to its direct anchor.
    fn check_adaptive_anchor_chain(&self, frontier: &S, direct_anchor: &C) -> Result<()>
    where
        S: Ord + Debug,
        C: Clone + Eq + Hash + Debug,
    {
        let anchors = self.adaptive_anchor_chain(direct_anchor, frontier);
        let previous = |candidate: &C| self.previous_certified(candidate, frontier);
        previous.check_predecessor_chain(direct_anchor.clone())?;
        ensure!(anchors.last() == Some(direct_anchor), "Adaptive chain does not end at its direct anchor");
        ensure!(
            anchors.iter().all(|anchor| &self.slot_of(anchor) > frontier),
            "Adaptive chain crosses finalized frontier {frontier:?}: {anchors:?}"
        );
        ensure!(
            anchors.windows(2).all(|pair| self.slot_of(&pair[0]) < self.slot_of(&pair[1])),
            "Adaptive chain is not strictly increasing: {anchors:?}"
        );
        ensure!(
            anchors.windows(2).all(|pair| self.previous_certified(&pair[1], frontier).as_ref() == Some(&pair[0])),
            "Adaptive chain does not follow certified predecessors: {anchors:?}"
        );
        ensure!(
            anchors.first().is_some_and(|anchor| self.previous_certified(anchor, frontier).is_none()),
            "Adaptive chain terminates before exhausting certified predecessors: {anchors:?}"
        );
        Ok(())
    }

    /// Checks that two oldest-first anchor histories are prefix-compatible.
    fn check_adaptive_prefix_compatibility(&self, left: &[C], right: &[C]) -> Result<()>
    where
        C: Eq + Debug,
    {
        ensure!(
            left.iter().zip(right).all(|(left, right)| left == right),
            "Adaptive anchor histories are not prefix-compatible: left={left:?}, right={right:?}"
        );
        Ok(())
    }
}

/// Checks adaptive direct-anchor legitimacy independently from predecessor-chain laws.
#[ext(name = AdaptiveDirectLawsExt)]
pub impl<This, S, C, V> This
where
    This: AdaptiveEvidence<S, C>
        + CandidateAuthor<Validator = V, Candidate = C>
        + CandidateSpace<Slot = S>
        + CommitteeOrder
        + ValidatorEvidence<CandidateSupport, C>
        + ValidatorSet,
{
    /// Checks that a selected direct anchor satisfies the shared strict-supermajority legitimacy predicate.
    fn check_adaptive_direct_legitimacy<Schedule>(&self, frontier: &S, schedule: &Schedule) -> Result<()>
    where
        Schedule: CandidateRestriction<S, C> + FollowingBoundary<S> + LeaderSelectionPolicy<S, V>,
        S: Clone + Ord + Debug,
        C: Clone + Debug,
        V: Clone + Eq,
    {
        ensure!(
            self.adaptive_direct_anchor(frontier, schedule).is_none_or(|anchor| self.two_thirds_legitimate(&anchor)),
            "Adaptive selected a direct anchor without strict-supermajority legitimacy"
        );
        Ok(())
    }
}

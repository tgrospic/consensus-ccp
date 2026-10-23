use crate::{CordialBlocklace, CordialCollapseExt, CordialLeader, CordialOrderExt, RatifiedPredecessor};
use alux_ext::ext;
use anyhow::{Result, ensure};
use ccp_core::{CandidateSpace, HorizonMatch, PredecessorChainLawsExt};
use std::{collections::BTreeSet, fmt::Debug, hash::Hash};

/// Checks Cordial Miners approved-fragment obligations.
#[ext(name = CordialOrderLawsExt)]
pub impl<This, C> This
where
    This: CordialBlocklace<Candidate = C>,
    C: Clone + Ord + Debug,
{
    /// Checks that `xsort` includes every eligible block once and respects causal dependencies.
    fn check_cordial_fragment(&self, anchor: &C, previous: Option<&C>) -> Result<()> {
        let fragment = self.cordial_fragment(anchor, previous);
        let eligible = self
            .blocks()
            .into_iter()
            .filter(|block| {
                self.observes(anchor, block)
                    && self.approves(anchor, block)
                    && previous.is_none_or(|previous| !self.observes(previous, block))
            })
            .collect::<BTreeSet<_>>();

        ensure!(
            fragment.len() == eligible.len() && fragment.iter().all(|block| eligible.contains(block)),
            "Cordial fragment omits or duplicates eligible blocks: fragment={fragment:?}, eligible={eligible:?}"
        );
        ensure!(
            fragment
                .iter()
                .enumerate()
                .all(|(index, block)| { fragment[index + 1..].iter().all(|later| !self.observes(block, later)) }),
            "Cordial fragment violates causal topological order: {fragment:?}"
        );
        Ok(())
    }
}

/// Checks Cordial Miners anchor-chain obligations.
#[ext(name = CordialLawsExt)]
pub impl<This, S, C> This
where
    This: CandidateSpace<Slot = S, Candidate = C> + CordialLeader + HorizonMatch<C, C> + RatifiedPredecessor,
{
    /// Checks that the history-relative chain exhausts ratified predecessors and strictly increases.
    fn check_cordial_anchor_chain(&self) -> Result<()>
    where
        S: Clone + Ord + Debug,
        C: Clone + Eq + Hash + Debug,
    {
        let anchors = self.cordial_finalizations();
        if let Some(anchor) = self.cordial_final_anchor() {
            let previous = |candidate: &C| self.previous_ratified(candidate);
            previous.check_predecessor_chain(anchor)?;
        }
        ensure!(
            anchors.windows(2).all(|pair| self.slot_of(&pair[0]) < self.slot_of(&pair[1])),
            "Cordial chain is not strictly increasing: {anchors:?}"
        );
        ensure!(
            anchors.windows(2).all(|pair| self.previous_ratified(&pair[1]).as_ref() == Some(&pair[0])),
            "Cordial chain does not follow ratified predecessors: {anchors:?}"
        );
        ensure!(
            anchors.first().is_none_or(|anchor| self.previous_ratified(anchor).is_none()),
            "Cordial chain terminates before exhausting ratified predecessors: {anchors:?}"
        );
        Ok(())
    }

    /// Checks the fragment of every finalized anchor, taken relative to the anchor finalized before it.
    fn check_cordial_fragments(&self) -> Result<()>
    where
        This: CordialBlocklace<Candidate = C>,
        S: Clone + Ord + Debug,
        C: Clone + Ord + Hash + Debug,
    {
        let anchors = self.cordial_finalizations();
        anchors.iter().enumerate().try_for_each(|(index, anchor)| {
            self.check_cordial_fragment(anchor, index.checked_sub(1).map(|previous| &anchors[previous]))
        })
    }

    /// Checks that an operational frontier only removes an already finalized prefix.
    fn check_cordial_frontier_suffix(&self, frontier: &S) -> Result<()>
    where
        S: Clone + Ord + Debug,
        C: Clone + Eq + Hash + Debug,
    {
        let expected = self
            .cordial_finalizations()
            .into_iter()
            .filter(|anchor| &self.slot_of(anchor) > frontier)
            .collect::<Vec<_>>();
        ensure!(
            self.cordial_finalizations_after(frontier) == expected,
            "Cordial frontier suffix is not derived from the history-relative prefix"
        );
        Ok(())
    }
}

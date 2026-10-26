use crate::{DenseLeaderEvidenceCollapseExt, SlotLeader};
use alux_ext::ext;
use anyhow::{Result, ensure};
use ccp_core::CandidateSpace;
use ccp_quorum::{CandidateLegitimacyExt, CandidateSupport, ValidatorEvidence};
use ccp_validator::{ValidatorSet, ValidatorWeight};
use std::{fmt::Debug, hash::Hash};

/// Checks legitimacy-driven leader-finalization obligations.
#[ext(name = DenseLeaderLawsExt)]
pub impl<This, S, C, V, W> This
where
    This: CandidateSpace<Slot = S, Candidate = C> + SlotLeader,
{
    /// Checks that the leaders finalized under `evidence` above `frontier` are exactly the
    /// legitimate slot leaders there, each leading its own slot, in strictly increasing slot order.
    fn check_dense_leader_finalizations<Evidence>(&self, evidence: &Evidence, frontier: &S) -> Result<()>
    where
        Evidence: ValidatorEvidence<CandidateSupport, C> + ValidatorSet<Validator = V> + ValidatorWeight<Weight = W>,
        V: Eq + Hash,
        W: Into<u128>,
        S: Clone + Ord + Debug,
        C: Clone + Eq + Hash + Debug,
    {
        let leaders = self.dense_leader_finalizations(evidence, frontier);
        ensure!(
            leaders.iter().all(|candidate| evidence.two_thirds_legitimate(candidate)),
            "Dense-leader chain includes a non-legitimate leader: {leaders:?}"
        );
        ensure!(
            leaders.iter().all(|candidate| self.slot_leader(&self.slot_of(candidate)).as_ref() == Some(candidate)),
            "Dense-leader chain includes a candidate that does not lead its slot: {leaders:?}"
        );
        ensure!(
            leaders.iter().all(|candidate| &self.slot_of(candidate) > frontier),
            "Dense-leader chain crosses frontier {frontier:?}: {leaders:?}"
        );
        ensure!(
            leaders.windows(2).all(|pair| self.slot_of(&pair[0]) < self.slot_of(&pair[1])),
            "Dense-leader chain is not strictly increasing: {leaders:?}"
        );
        ensure!(
            self.slots()
                .into_iter()
                .filter(|slot| slot > frontier)
                .filter_map(|slot| self.slot_leader(&slot))
                .all(|candidate| !evidence.two_thirds_legitimate(&candidate) || leaders.contains(&candidate)),
            "Dense-leader chain omits a legitimate leader above {frontier:?}: {leaders:?}"
        );
        Ok(())
    }
}

use crate::{MysticetiEvidenceViewExt, MysticetiSkip, RepairAnchorSlots};
use alux_ext::ext;
use anyhow::{Result, ensure};
use ccp_core::{CandidateSpace, HorizonMatch};
use ccp_quorum::ValidatorEvidence;
use ccp_validator::{ValidatorSet, ValidatorWeight};
use std::{fmt::Debug, hash::Hash};

/// Checks Mysticeti direct-evidence obligations.
#[ext(name = MysticetiLawsExt)]
pub impl<This, S, C, V, W> This
where
    This: CandidateSpace<Slot = S, Candidate = C>,
{
    /// Checks that no proposer slot carries incompatible direct commit and skip evidence.
    fn check_mysticeti_direct_exclusivity<Evidence>(&self, evidence: &Evidence) -> Result<()>
    where
        Evidence: HorizonMatch<C, C> + ValidatorEvidence<MysticetiSkip, S>
            + ValidatorSet<Validator = V>
            + ValidatorWeight<Weight = W>,
        V: Eq + Hash,
        W: Into<u128>,
        S: Debug,
        C: Clone + Debug,
    {
        self.slots().into_iter().try_for_each(|slot| {
            let commits = self.mysticeti_direct_commits(evidence, &slot);
            ensure!(
                commits.len() <= 1,
                "Mysticeti slot has conflicting direct commit evidence: slot={slot:?}, candidates={commits:?}"
            );
            ensure!(
                !self.mysticeti_direct_skip(evidence, &slot) || commits.is_empty(),
                "Mysticeti slot has both direct commit and skip evidence: {slot:?}"
            );
            Ok(())
        })
    }
}

/// Checks Mysticeti repair-schedule obligations.
#[ext(name = MysticetiRepairLawsExt)]
pub impl<This, S, C> This
where
    This: CandidateSpace<Slot = S, Candidate = C> + RepairAnchorSlots,
{
    /// Checks that repair anchors are later than their slot and strictly ordered.
    fn check_mysticeti_repair_order(&self) -> Result<()>
    where
        S: Ord + Debug,
    {
        self.slots().into_iter().try_for_each(|slot| {
            let anchors = self.repair_anchor_slots(&slot);
            ensure!(
                anchors.iter().all(|anchor| anchor > &slot) && anchors.windows(2).all(|pair| pair[0] < pair[1]),
                "Mysticeti repair anchors are not strictly ordered after {slot:?}: {anchors:?}"
            );
            Ok(())
        })
    }
}

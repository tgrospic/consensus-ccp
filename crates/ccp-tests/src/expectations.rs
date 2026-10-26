//! Checks the fixed expectations of concrete scenarios against the specification capabilities.
use alux_ext::ext;
use anyhow::{Result, ensure};
use ccp_communication::{
    HorizonMatchEvidence, JustificationRelation, MessageCarrier, MessageCarrierExt, MessageJustification,
    MessageJustificationExt, MessageSlot, PreviousRoundWitnesses, UncontestedRelation,
};
use ccp_cordial::{
    CordialCollapseExt, CordialEvidenceExt, CordialLawsExt, CordialLeader, CordialLeaderExt, RatifiedPredecessor,
};
use ccp_core::{
    CandidateSpace, Closed, CommitMatch, EvidenceRelation, FollowingBoundary, HistoryCollapse, HorizonMatch,
    MatchedCollapseExt,
};
use ccp_mysticeti::MysticetiCertificateExt;
use ccp_quorum::QuorumOf;
use ccp_validator::{TwoThirdsQuorum, ValidatorSet, ValidatorWeight};
use std::{fmt::Debug, hash::Hash};

/// Checks retained carrier observations after constructive expansion.
#[ext(name = MessageCarrierExpectationExt)]
pub impl<This, V, C> This
where
    This: MessageCarrier<Validator = V, Message = C>,
    V: Clone,
    C: Clone + Eq,
{
    /// Checks the retained messages and, optionally, all candidates at one slot.
    fn check_retention(&self, expected_messages: &[C], slot_case: Option<(&MessageSlot<V>, &[C])>) -> Result<()>
    where
        C: Debug,
    {
        let messages = self.messages();
        ensure!(messages == expected_messages, "carrier retained {messages:?}, expected {expected_messages:?}");
        if let Some((slot, expected)) = slot_case {
            let candidates = self.slot_messages(slot);
            ensure!(candidates == expected, "slot retained {candidates:?}, expected {expected:?}");
        }
        Ok(())
    }

    /// Checks every candidate retained at one communication slot.
    fn check_slot_retention(&self, slot: &MessageSlot<V>, expected: &[C]) -> Result<()>
    where
        C: Debug,
    {
        let candidates = self.slot_messages(slot);
        ensure!(candidates == expected, "slot retained {candidates:?}, expected {expected:?}");
        Ok(())
    }
}

/// Checks the shared evidence algebra from observation through collapse continuation.
#[ext(name = CommonEvidenceExpectationExt)]
pub impl<This, V, W, C> This
where
    This: MessageCarrier<Validator = V, Message = C> + MessageJustification + ValidatorSet + ValidatorWeight<Weight = W>,
    V: Clone + Eq + Hash,
    C: Clone + Debug + Eq + Hash,
    W: Into<u128>,
{
    /// Checks that approval quorum forms, matches its horizon, and commits the matched candidate.
    fn check_positive_evidence_composition<Boundary>(
        &self,
        boundary: &Boundary,
        witness: &C,
        candidate: &C,
    ) -> Result<()>
    where
        Boundary: FollowingBoundary<u64>,
    {
        let observation = JustificationRelation::new(self);
        let approval = UncontestedRelation::new(self, observation);
        let ratification = QuorumOf::new(PreviousRoundWitnesses::new(self), approval, TwoThirdsQuorum);
        ensure!(ratification.relates(witness, candidate), "witness does not carry an approval quorum");

        let horizon = HorizonMatchEvidence::new(self, boundary, ratification);
        ensure!(
            horizon.match_horizon(candidate).as_ref() == Some(candidate),
            "approval quorum does not match the candidate's evidence horizon"
        );
        ensure!(
            horizon.collapse_when_matched(candidate, &CommitMatch) == Some(Closed::Commit(candidate.clone())),
            "a successful horizon match does not continue to candidate commitment"
        );
        Ok(())
    }
}

/// Checks a table of history-relative collapse classifications.
#[ext(name = HistoryCollapseExpectationExt)]
pub impl<This> This {
    /// Checks every query and expected partial observation in one semantic scenario.
    fn check_collapse_cases<Collapse, Q, P>(&self, collapse: &Collapse, cases: &[(Q, Option<P>)]) -> Result<()>
    where
        Collapse: HistoryCollapse<This, Q, P>,
        Q: Debug,
        P: Debug + Eq,
    {
        cases.iter().try_for_each(|(query, expected)| {
            let actual = collapse.collapse_history(self, query);
            ensure!(
                actual.as_ref() == expected.as_ref(),
                "collapse at {query:?} produced {actual:?}, expected {expected:?}"
            );
            Ok(())
        })
    }
}

/// Checks agreement between two horizon interpretations of the same query.
#[ext(name = HorizonAgreementExpectationExt)]
pub impl<This, Q, M> This
where
    This: HorizonMatch<Q, M>,
{
    /// Checks that both interpretations positively select the same horizon witness.
    fn check_positive_horizon_agreement<Other>(&self, other: &Other, query: &Q) -> Result<()>
    where
        Other: HorizonMatch<Q, M>,
        M: Debug + Eq,
    {
        let left = self.match_horizon(query);
        let right = other.match_horizon(query);
        ensure!(left.is_some(), "left horizon interpretation did not match: {left:?}");
        ensure!(left == right, "horizon interpretations disagree: left={left:?}, right={right:?}");
        Ok(())
    }
}

/// Checks Cordial vocabulary derived from the shared communication algebra.
#[ext(name = CordialEvidenceExpectationExt)]
pub impl<This, V, W, C> This
where
    This: MessageCarrier<Validator = V, Message = C> + MessageJustification + ValidatorSet + ValidatorWeight<Weight = W>,
    V: Clone + Eq + Hash,
    C: Clone + Debug + Eq + Hash,
    W: Into<u128>,
{
    /// Checks ratification and successive super-ratification witnesses in one history.
    fn check_cordial_evidence<Boundary>(&self, witness: &C, candidate: &C, later: &C, boundary: &Boundary) -> Result<()>
    where
        Boundary: FollowingBoundary<u64>,
    {
        ensure!(self.cordial_ratifies(witness, candidate), "{witness:?} does not ratify {candidate:?}");
        ensure!(self.cordial_super_ratified_at(candidate, boundary), "{candidate:?} is not super-ratified");
        ensure!(self.cordial_super_ratified_at(later, boundary), "{later:?} is not super-ratified");
        Ok(())
    }
}

/// Checks the Cordial finalized-anchor interpretation through its primitive capabilities.
#[ext(name = CordialAnchorExpectationExt)]
pub impl<This, S, C> This
where
    This: CandidateSpace<Slot = S, Candidate = C> + CordialLeader + HorizonMatch<C, C> + RatifiedPredecessor,
    S: Clone + Debug + Ord,
    C: Clone + Debug + Eq + Hash,
{
    /// Checks two final leaders, their predecessor edge, and the complete anchor-chain laws.
    fn check_cordial_anchor_case(&self, first_slot: &S, later_slot: &S, first: &C, later: &C) -> Result<()> {
        ensure!(self.cordial_final_leader(first_slot).as_ref() == Some(first), "first leader is not final");
        ensure!(self.cordial_final_leader(later_slot).as_ref() == Some(later), "later leader is not final");
        ensure!(self.previous_ratified(later).as_ref() == Some(first), "ratified predecessor is incorrect");
        ensure!(
            self.cordial_finalizations() == [first.clone(), later.clone()],
            "finalized Cordial anchor chain is incomplete"
        );
        self.check_cordial_anchor_chain()
    }
}

/// Checks Mysticeti certificates derived from shared communication capabilities.
#[ext(name = MysticetiCertificateExpectationExt)]
pub impl<This, V, W, C> This
where
    This: MessageCarrier<Validator = V, Message = C> + MessageJustification + ValidatorSet + ValidatorWeight<Weight = W>,
    V: Clone + Eq + Hash,
    C: Clone + Debug + Eq + Hash,
    W: Into<u128>,
{
    /// Checks certificate formation, causal visibility, and the stronger certified-link relation.
    fn check_mysticeti_certificate<Boundary>(
        &self,
        certificate: &C,
        non_certificate: &C,
        anchor: &C,
        candidate: &C,
        boundary: &Boundary,
        expected_link: bool,
    ) -> Result<()>
    where
        Boundary: FollowingBoundary<u64>,
    {
        ensure!(self.mysticeti_certifies(certificate, candidate), "expected certificate is absent");
        ensure!(!self.mysticeti_certifies(non_certificate, candidate), "non-certificate was accepted");
        ensure!(self.justification_observes(anchor, candidate), "anchor does not causally observe the candidate");
        ensure!(
            self.mysticeti_certified_link(anchor, candidate, boundary) == expected_link,
            "certified-link classification disagrees with the certificate visible to the anchor"
        );
        Ok(())
    }
}

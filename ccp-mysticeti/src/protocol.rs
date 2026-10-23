//! Specifies Mysticeti's dense fixed-horizon collapse and repair policy.
use alux_ext::ext;
use ccp_communication::{
    HorizonMatchEvidence, JustificationRelation, MessageCarrier, MessageCarrierExt, MessageJustification,
    MessageJustificationExt, MessageSlot, PreviousRoundWitnesses, UncontestedRelation,
};
use ccp_core::CandidateSorts;
use ccp_core::{
    AllCandidates, CandidateSpace, Decision, EvidenceRelation, FixedDistance, FollowingBoundary, HorizonMatch,
};
use ccp_leader::{PeriodicRoundRobin, Policy};
use ccp_quorum::ValidatorSorts;
use ccp_quorum::{CandidateSupport, QuorumOf, TwoThirdsQuorum, ValidatorEvidence, ValidatorQuorumExt, ValidatorSet};
use derive_new::new as New;
use std::hash::Hash;
use std::marker::PhantomData;

/// Identifies validator evidence for Mysticeti's negative direct-decision pattern.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct MysticetiSkip;

/// Names the shared uncontested justification relation in Mysticeti's vocabulary.
pub type MysticetiVote<'a, Carrier, V> = UncontestedRelation<'a, Carrier, JustificationRelation<'a, Carrier>, V>;

/// Names a quorum of Mysticeti votes as a certificate.
pub type MysticetiCertificate<'a, Carrier, V> =
    QuorumOf<PreviousRoundWitnesses<'a, Carrier>, MysticetiVote<'a, Carrier, V>, TwoThirdsQuorum>;

/// Derives Mysticeti certificate and certified-link relations from a message DAG.
#[ext(name = MysticetiCertificateExt)]
pub impl<This, V, C> This
where
    This: MessageCarrier<Validator = V, Message = C> + MessageJustification + ValidatorSet,
    V: Clone + Eq + Hash,
    C: Clone + Eq + Hash,
{
    /// Reports whether `witness` is a certificate for `candidate`.
    fn mysticeti_certifies(&self, witness: &C, candidate: &C) -> bool {
        let observation = JustificationRelation::new(self);
        let vote = UncontestedRelation::new(self, observation);
        QuorumOf::new(PreviousRoundWitnesses::new(self), vote, TwoThirdsQuorum).relates(witness, candidate)
    }

    /// Reports whether `anchor` observes a certificate from `candidate`'s decision round.
    fn mysticeti_certified_link<Boundary>(&self, anchor: &C, candidate: &C, boundary: &Boundary) -> bool
    where
        Boundary: FollowingBoundary<u64>,
    {
        self.message_slot_of(candidate).is_some_and(|candidate_slot| {
            let decision_round = boundary.following_boundary(&candidate_slot.round);
            self.message_slots()
                .into_iter()
                .filter(|slot| slot.round == decision_round)
                .flat_map(|slot| self.slot_messages(&slot))
                .any(|witness| {
                    self.mysticeti_certifies(&witness, candidate) && self.justification_observes(anchor, &witness)
                })
        })
    }
}

/// Exposes the ordered later anchor slots eligible to repair an undecided direct slot.
pub trait RepairAnchorSlots: CandidateSorts {
    /// Returns repair anchors after `slot`'s decision round, earliest first.
    fn repair_anchor_slots(&self, slot: &Self::Slot) -> Vec<Self::Slot>;
}

/// Exposes certified causal links from a final anchor to an earlier candidate.
pub trait CertifiedLink: CandidateSorts {
    /// Reports whether `anchor` carries a certificate link to `candidate`.
    fn certified_link(&self, anchor: &Self::Candidate, candidate: &Self::Candidate) -> bool;
}

/// Describes Mysticeti candidate retention, direct horizon, and leader schedule.
pub type MysticetiPolicy = Policy<AllCandidates, FixedDistance, PeriodicRoundRobin>;

/// Builds the policy consumed by Mysticeti DAG evidence and comparison scenarios.
pub fn mysticeti_policy(direct_distance: u64) -> MysticetiPolicy {
    Policy::new(AllCandidates, FixedDistance::new(direct_distance), PeriodicRoundRobin::new(0, 1))
}

/// Interprets Mysticeti support and skip evidence from message justifications.
#[derive(Debug, Clone, Copy, New)]
pub struct MysticetiDagEvidence<'a, Carrier, Schedule, C> {
    carrier: &'a Carrier,
    schedule: &'a Schedule,
    #[new(default)]
    candidate: PhantomData<C>,
}

impl<Carrier, Schedule, C> ValidatorSorts for MysticetiDagEvidence<'_, Carrier, Schedule, C>
where
    Carrier: ValidatorSorts,
{
    type Validator = Carrier::Validator;
}

impl<Carrier, Schedule, C> ValidatorSet for MysticetiDagEvidence<'_, Carrier, Schedule, C>
where
    Carrier: ValidatorSet,
{
    fn validators(&self) -> Vec<Self::Validator> {
        self.carrier.validators()
    }
}

impl<Carrier, Schedule, V, C> ValidatorEvidence<CandidateSupport, C> for MysticetiDagEvidence<'_, Carrier, Schedule, C>
where
    Carrier:
        MessageCarrier<Validator = V, Message = C> + MessageJustification<Message = C> + ValidatorSet<Validator = V>,
    Schedule: FollowingBoundary<u64>,
    V: Clone + Eq + Hash,
    C: Clone + Eq + Hash,
{
    fn supports(&self, _kind: &CandidateSupport, validator: &V, candidate: &C) -> bool {
        let observation = JustificationRelation::new(self.carrier);
        let vote = UncontestedRelation::new(self.carrier, observation);
        let certificate = QuorumOf::new(PreviousRoundWitnesses::new(self.carrier), vote, TwoThirdsQuorum);
        HorizonMatchEvidence::new(self.carrier, self.schedule, certificate).supports(
            &CandidateSupport,
            validator,
            candidate,
        )
    }
}

impl<Carrier, Schedule, C> HorizonMatch<C, C> for MysticetiDagEvidence<'_, Carrier, Schedule, C>
where
    Carrier: MessageCarrier<Message = C> + MessageJustification + ValidatorSet,
    Schedule: FollowingBoundary<u64>,
    Carrier::Validator: Clone + Eq + Hash,
    C: Clone + Eq + Hash,
{
    fn match_horizon(&self, query: &C) -> Option<C> {
        let observation = JustificationRelation::new(self.carrier);
        let vote = UncontestedRelation::new(self.carrier, observation);
        let certificate = QuorumOf::new(PreviousRoundWitnesses::new(self.carrier), vote, TwoThirdsQuorum);
        HorizonMatchEvidence::new(self.carrier, self.schedule, certificate).match_horizon(query)
    }
}

impl<Carrier, Schedule, V, C> ValidatorEvidence<MysticetiSkip, MessageSlot<V>>
    for MysticetiDagEvidence<'_, Carrier, Schedule, C>
where
    Carrier:
        MessageCarrier<Validator = V, Message = C> + MessageJustification<Message = C> + ValidatorSet<Validator = V>,
    V: Clone + Eq,
    C: Clone + Eq,
{
    fn supports(&self, _kind: &MysticetiSkip, validator: &V, target: &MessageSlot<V>) -> bool {
        let vote_slot = MessageSlot::new(target.round.saturating_add(1), validator.clone());
        self.carrier.message_at(&vote_slot).is_some_and(|vote| {
            let proposals = self.carrier.slot_messages(target);
            !proposals.is_empty()
                && proposals.iter().all(|proposal| !self.carrier.message_justifications(&vote).contains(proposal))
        })
    }
}

/// Derives Mysticeti evidence patterns from a separately interpreted evidence view.
#[ext(name = MysticetiEvidenceViewExt)]
pub impl<This, S, C> This
where
    This: CandidateSpace<Slot = S, Candidate = C>,
    C: Clone,
{
    /// Returns every candidate in `slot` carrying direct-commit evidence from more than 2/3.
    fn mysticeti_direct_commits<Evidence>(&self, evidence: &Evidence, slot: &S) -> Vec<C>
    where
        Evidence: HorizonMatch<C, C>,
    {
        self.candidates(slot).into_iter().filter_map(|candidate| evidence.match_horizon(&candidate)).collect()
    }

    /// Returns the uniquely supported candidate in `slot`.
    fn mysticeti_direct_commit<Evidence>(&self, evidence: &Evidence, slot: &S) -> Option<C>
    where
        Evidence: HorizonMatch<C, C>,
    {
        let mut candidates = self.mysticeti_direct_commits(evidence, slot).into_iter();
        candidates.next().filter(|_| candidates.next().is_none())
    }

    /// Reports whether `slot` carries direct-skip evidence from more than 2/3.
    fn mysticeti_direct_skip<Evidence, V>(&self, evidence: &Evidence, slot: &S) -> bool
    where
        Evidence: ValidatorEvidence<MysticetiSkip, S> + ValidatorSet<Validator = V>,
    {
        evidence.has_quorum(&TwoThirdsQuorum, &MysticetiSkip, slot)
    }
}

/// Derives Mysticeti's direct decision from a separately interpreted evidence view.
#[ext(name = MysticetiDirectViewExt)]
pub impl<This, S, C> This
where
    This: CandidateSpace<Slot = S, Candidate = C>,
    S: Clone,
    C: Clone,
{
    /// Classifies one slot from the evidence view's skip and commit patterns.
    fn mysticeti_direct_decision<Evidence, V>(&self, evidence: &Evidence, slot: &S) -> Decision<S, C>
    where
        Evidence: HorizonMatch<C, C> + ValidatorEvidence<MysticetiSkip, S> + ValidatorSet<Validator = V>,
    {
        if self.mysticeti_direct_skip(evidence, slot) {
            return Decision::Skip(slot.clone());
        }
        self.mysticeti_direct_commit(evidence, slot).map_or_else(|| Decision::Undecided(slot.clone()), Decision::Commit)
    }
}

/// Derives Mysticeti collapse from separately interpreted direct evidence.
#[ext(name = MysticetiCollapseViewExt)]
pub impl<This, S, C> This
where
    This: CandidateSpace<Slot = S, Candidate = C> + RepairAnchorSlots + CertifiedLink,
    S: Clone,
    C: Clone,
{
    /// Classifies one slot using direct evidence followed by ordered anchor repair.
    fn mysticeti_decision<Evidence, V>(&self, evidence: &Evidence, slot: &S) -> Decision<S, C>
    where
        Evidence: HorizonMatch<C, C> + ValidatorEvidence<MysticetiSkip, S> + ValidatorSet<Validator = V>,
    {
        let direct = self.mysticeti_direct_decision(evidence, slot);
        if direct.is_closed() {
            return direct;
        }

        for anchor_slot in self.repair_anchor_slots(slot) {
            match self.mysticeti_direct_decision(evidence, &anchor_slot) {
                Decision::Commit(anchor) => {
                    return self
                        .candidates(slot)
                        .into_iter()
                        .find(|candidate| self.certified_link(&anchor, candidate))
                        .map_or_else(|| Decision::Skip(slot.clone()), Decision::Commit);
                }
                Decision::Undecided(_) => return Decision::Undecided(slot.clone()),
                Decision::Skip(_) => {}
            }
        }
        Decision::Undecided(slot.clone())
    }

    /// Returns the closed prefix under the supplied evidence view.
    fn mysticeti_closed_prefix<Evidence, V>(&self, evidence: &Evidence) -> Vec<Decision<S, C>>
    where
        Evidence: HorizonMatch<C, C> + ValidatorEvidence<MysticetiSkip, S> + ValidatorSet<Validator = V>,
    {
        self.slots()
            .into_iter()
            .map(|slot| self.mysticeti_decision(evidence, &slot))
            .take_while(Decision::is_closed)
            .collect()
    }
}

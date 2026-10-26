//! Specifies the Cordial Miners sparse fixed-wave collapse policy.
use alux_ext::ext;
use ccp_communication::{
    HorizonMatchEvidence, JustificationRelation, MessageCarrier, MessageJustification, PreviousRoundWitnesses,
    UncontestedRelation,
};
use ccp_core::CandidateSorts;
use ccp_core::{
    CandidateSpace, Decision, EveryNthSlot, EvidenceRelation, FixedDistance, FollowingBoundary, HorizonMatch,
    PredecessorChainExt,
};
use ccp_leader::{PeriodicRoundRobin, Policy};
use ccp_quorum::QuorumOf;
use ccp_validator::{TwoThirdsQuorum, ValidatorSet, ValidatorWeight};
use std::collections::BTreeSet;
use std::hash::Hash;

/// Names uncontested justification in Cordial Miners' approval vocabulary.
pub type CordialApproval<'a, Carrier, V> = UncontestedRelation<'a, Carrier, JustificationRelation<'a, Carrier>, V>;

/// Names a quorum of Cordial approvals as ratification.
pub type CordialRatification<'a, Carrier, V> =
    QuorumOf<PreviousRoundWitnesses<'a, Carrier>, CordialApproval<'a, Carrier, V>, TwoThirdsQuorum>;

/// Exposes the Cordial leader candidate occupying a scheduled wave slot.
pub trait CordialLeader: CandidateSorts {
    /// Returns the leader candidate occupying `slot`, when one was produced.
    fn cordial_leader(&self, slot: &Self::Slot) -> Option<Self::Candidate>;
}

/// Exposes the latest earlier leader ratified by a final or recursively selected leader.
pub trait RatifiedPredecessor: CandidateSorts {
    /// Returns the latest earlier leader ratified in `anchor`'s causal past.
    fn previous_ratified(&self, anchor: &Self::Candidate) -> Option<Self::Candidate>;
}

/// Exposes Cordial Miners observation and approval relations over the blocklace.
pub trait CordialBlocklace: CandidateSorts {
    /// Returns every retained block.
    fn blocks(&self) -> Vec<Self::Candidate>;

    /// Reports whether `viewer` observes `block`.
    fn observes(&self, viewer: &Self::Candidate, block: &Self::Candidate) -> bool;

    /// Reports whether `viewer` approves `block` without observing an equivocation.
    fn approves(&self, viewer: &Self::Candidate, block: &Self::Candidate) -> bool;
}

/// Describes Cordial Miners candidate, horizon, and leader scheduling for scenario construction.
pub type CordialPolicy = Policy<EveryNthSlot, FixedDistance, PeriodicRoundRobin>;

/// Builds the comparison policy used to construct sparse fixed-wave Cordial evidence.
pub fn cordial_policy(first: u64, wave: u64) -> CordialPolicy {
    Policy::new(
        EveryNthSlot::new(first, wave),
        FixedDistance::new(wave.saturating_sub(1)),
        PeriodicRoundRobin::new(first, wave),
    )
}

/// Presents common evidence relations in Cordial Miners' vocabulary.
#[ext(name = CordialEvidenceExt)]
pub impl<This, V, W, C> This
where
    This:
        MessageCarrier<Validator = V, Message = C> + MessageJustification + ValidatorSet + ValidatorWeight<Weight = W>,
    V: Clone + Eq + Hash,
    W: Into<u128>,
    C: Clone + Eq + Hash,
{
    /// Reports whether `witness` ratifies `candidate` through a quorum of approvals.
    fn cordial_ratifies(&self, witness: &C, candidate: &C) -> bool {
        let observation = JustificationRelation::new(self);
        let approval = UncontestedRelation::new(self, observation);
        QuorumOf::new(PreviousRoundWitnesses::new(self), approval, TwoThirdsQuorum).relates(witness, candidate)
    }

    /// Reports whether a wave horizon matches a super-ratification witness for `candidate`.
    fn cordial_super_ratified_at<Boundary>(&self, candidate: &C, boundary: &Boundary) -> bool
    where
        Boundary: FollowingBoundary<u64>,
    {
        let observation = JustificationRelation::new(self);
        let approval = UncontestedRelation::new(self, observation);
        let ratification = QuorumOf::new(PreviousRoundWitnesses::new(self), approval, TwoThirdsQuorum);
        HorizonMatchEvidence::new(self, boundary, ratification).match_horizon(candidate).is_some()
    }
}

/// Derives fixed-wave final leaders from typed validator evidence.
#[ext(name = CordialLeaderExt)]
pub impl<This, S, C> This
where
    This: CordialLeader<Slot = S, Candidate = C> + HorizonMatch<C, C>,
    C: Clone,
{
    /// Returns the leader in `slot` when more than 2/3 of validators witness its finality.
    fn cordial_final_leader(&self, slot: &S) -> Option<C> {
        self.cordial_leader(slot).and_then(|leader| self.match_horizon(&leader))
    }
}

/// Derives Cordial Miners decisions from final leaders and ratified predecessors.
#[ext(name = CordialCollapseExt)]
pub impl<This, S, C> This
where
    This: CandidateSpace<Slot = S, Candidate = C> + CordialLeader + HorizonMatch<C, C> + RatifiedPredecessor,
    S: Clone + Ord,
    C: Clone + Eq + Hash,
{
    /// Returns the newest final leader determined by the current history.
    fn cordial_final_anchor(&self) -> Option<C> {
        self.slots().into_iter().rev().find_map(|slot| self.cordial_final_leader(&slot))
    }

    /// Returns the complete backward-repaired ratified-leader chain, oldest first.
    fn cordial_anchor_chain(&self, final_anchor: &C) -> Vec<C> {
        let previous = |anchor: &C| self.previous_ratified(anchor);
        previous.predecessor_chain(final_anchor.clone())
    }

    /// Returns the complete finalized leader prefix determined by the current history.
    fn cordial_finalizations(&self) -> Vec<C> {
        self.cordial_final_anchor().map_or_else(Vec::new, |anchor| self.cordial_anchor_chain(&anchor))
    }

    /// Returns the finalized leader suffix strictly above an operational frontier.
    fn cordial_finalizations_after(&self, frontier: &S) -> Vec<C> {
        self.cordial_finalizations().into_iter().filter(|anchor| &self.slot_of(anchor) > frontier).collect()
    }

    /// Classifies a slot from the complete history-relative finalized prefix.
    fn cordial_decision(&self, slot: &S) -> Decision<S, C> {
        let chain = self.cordial_finalizations();
        if let Some(candidate) = chain.iter().find(|candidate| &self.slot_of(candidate) == slot).cloned() {
            Decision::Commit(candidate)
        } else if chain.last().is_some_and(|anchor| slot <= &self.slot_of(anchor)) {
            Decision::Skip(slot.clone())
        } else {
            Decision::Undecided(slot.clone())
        }
    }

    /// Returns the complete closed prefix, stopping at the first unresolved slot.
    fn cordial_closed_prefix(&self) -> Vec<Decision<S, C>> {
        self.slots().into_iter().map(|slot| self.cordial_decision(&slot)).take_while(Decision::is_closed).collect()
    }

    /// Classifies a slot only when it lies strictly above an operational frontier.
    fn cordial_decision_after(&self, slot: &S, frontier: &S) -> Decision<S, C> {
        if slot > frontier { self.cordial_decision(slot) } else { Decision::Undecided(slot.clone()) }
    }
}

/// Derives Cordial Miners' equivocation-excluding blocklace fragments.
#[ext(name = CordialOrderExt)]
pub impl<This, C> This
where
    This: CordialBlocklace<Candidate = C>,
    C: Clone + Ord,
{
    /// Returns `xsort(anchor, [anchor] \ [previous])` in deterministic topological order.
    fn cordial_fragment(&self, anchor: &C, previous: Option<&C>) -> Vec<C> {
        let mut remaining = self
            .blocks()
            .into_iter()
            .filter(|block| {
                self.observes(anchor, block)
                    && self.approves(anchor, block)
                    && previous.is_none_or(|previous| !self.observes(previous, block))
            })
            .collect::<BTreeSet<_>>();
        let mut ordered = Vec::with_capacity(remaining.len());

        while let Some(block) = remaining
            .iter()
            .find(|block| !remaining.iter().any(|dependency| block != &dependency && self.observes(block, dependency)))
            .cloned()
        {
            remaining.remove(&block);
            ordered.push(block);
        }

        ordered
    }

    /// Returns ordered fragments for an oldest-first anchor chain.
    fn cordial_fragments(&self, anchors: &[C], previous: Option<&C>) -> Vec<Vec<C>> {
        let mut previous = previous.cloned();
        anchors
            .iter()
            .map(|anchor| {
                let fragment = self.cordial_fragment(anchor, previous.as_ref());
                previous = Some(anchor.clone());
                fragment
            })
            .collect()
    }
}

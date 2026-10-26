//! Checks independent evidence choices and realization of a composed direct-commit rule.
use crate::scenario::{Expansion, History, expansion, genesis};
use anyhow::Result;
use ccp_communication::{
    JustificationRelation, MessageCarrier, MessageCarrierExt, MessageInput, MessageJustification, MessageSlot,
    PreviousRoundWitnesses,
};
use ccp_core::CollapseSorts;
use ccp_core::{
    CollapsePolicy, CollapsePolicyExt, CollapseRule, Denotation, EvidenceDenotation, EvidenceExtraction,
    EvidenceRelation, HistoryDenotation, HorizonMatch, HorizonProjection, OutputDenotation, RealizationLawsExt,
    VisibleDenotation,
};
use ccp_mysticeti::{MysticetiCertificateExt, MysticetiDagEvidence, mysticeti_policy};
use ccp_quorum::{QuorumOf, WitnessSelection};
use ccp_runtime::{CandidateId, Machine, ValidatorId};
use ccp_validator::{
    QuorumThreshold, QuorumThresholdExt, TwoThirdsQuorum, ValidatorSet, ValidatorSorts, ValidatorWeight,
};
use std::collections::BTreeSet;
use std::marker::PhantomData;

type TestMachine = Machine<History, Expansion, ()>;

fn append(machine: &mut TestMachine, round: u64, validator: u64, candidate: u64, parents: &[u64]) {
    machine
        .apply(&MessageInput::new(
            MessageSlot::new(round, ValidatorId(validator)),
            CandidateId(candidate),
            parents.iter().copied().map(CandidateId).collect(),
        ))
        .unwrap();
}

// An alternate scope with no previous-round restriction; the quorum derivation is unchanged.
struct DirectJustifications<'a>(&'a History);

impl ValidatorSorts for DirectJustifications<'_> {
    type Validator = ValidatorId;
    type Weight = u64;
}

impl ValidatorSet for DirectJustifications<'_> {
    fn validators(&self) -> Vec<ValidatorId> {
        self.0.validators()
    }
}

impl ValidatorWeight for DirectJustifications<'_> {
    fn weight(&self, validator: &ValidatorId) -> u64 {
        self.0.weight(validator)
    }
}

impl WitnessSelection<CandidateId> for DirectJustifications<'_> {
    type Evidence = CandidateId;

    fn witnesses(&self, witness: &CandidateId) -> Vec<(ValidatorId, CandidateId)> {
        self.0
            .message_justifications(witness)
            .into_iter()
            .filter_map(|message| self.0.message_slot_of(&message).map(|slot| (slot.validator, message)))
            .collect()
    }
}

struct OneThird;

impl QuorumThreshold for OneThird {
    fn threshold_numerator(&self) -> u64 {
        1
    }
    fn threshold_denominator(&self) -> u64 {
        3
    }
}

#[test]
fn witness_scope_threshold_and_distinct_authors_are_independent() {
    let mut machine = Machine::new(genesis(), expansion(), ());
    for validator in 0..4 {
        append(&mut machine, 0, validator, validator, &[]);
    }
    append(&mut machine, 1, 0, 10, &[0, 1, 2]);
    append(&mut machine, 1, 1, 11, &[0, 1, 2]);
    append(&mut machine, 1, 2, 12, &[1, 2, 3]);
    // One author equivocates; its two supporting messages still count only once.
    append(&mut machine, 1, 0, 100, &[0, 1, 2]);
    append(&mut machine, 2, 3, 20, &[10, 11, 12, 100, 10, 0]);
    let history = machine.history();
    let previous =
        QuorumOf::new(PreviousRoundWitnesses::new(history), JustificationRelation::new(history), TwoThirdsQuorum);
    let lower = QuorumOf::new(PreviousRoundWitnesses::new(history), JustificationRelation::new(history), OneThird);
    let direct = QuorumOf::new(DirectJustifications(history), JustificationRelation::new(history), TwoThirdsQuorum);
    assert!(!previous.relates(&CandidateId(20), &CandidateId(0)));
    assert!(lower.relates(&CandidateId(20), &CandidateId(0)));
    // The older message is by validator 0, already counted: scope alone cannot inflate support.
    assert!(!direct.relates(&CandidateId(20), &CandidateId(0)));
    // An older message by a third author supplies that author's independent support for target 3.
    append(&mut machine, 2, 2, 21, &[10, 11, 12, 3]);
    let history = machine.history();
    let previous = QuorumOf::new(PreviousRoundWitnesses::new(history), JustificationRelation::new(history), OneThird);
    let direct = QuorumOf::new(DirectJustifications(history), JustificationRelation::new(history), OneThird);
    assert!(!previous.relates(&CandidateId(21), &CandidateId(3)));
    assert!(direct.relates(&CandidateId(21), &CandidateId(3)));
}

// L retains the author and round of every certificate for one target.
struct Certificates(CandidateId);
impl CollapseSorts for Certificates {
    type History = History;
    type Evidence = Vec<MessageSlot<ValidatorId>>;
    type Visible = Vec<ValidatorId>;
    type Output = CandidateId;
}

impl EvidenceExtraction for Certificates {
    fn extract_evidence(&self, history: &History) -> Vec<MessageSlot<ValidatorId>> {
        history
            .messages()
            .into_iter()
            .filter(|witness| history.mysticeti_certifies(witness, &self.0))
            .filter_map(|witness| history.message_slot_of(&witness))
            .collect()
    }
}

// R selects certificate authors at a particular boundary.
struct AtRound<Sorts>(u64, PhantomData<Sorts>);
impl<Sorts: CollapseSorts> CollapseSorts for AtRound<Sorts> {
    type History = Sorts::History;
    type Evidence = Sorts::Evidence;
    type Visible = Sorts::Visible;
    type Output = Sorts::Output;
}
impl HorizonProjection for AtRound<Certificates> {
    fn project_horizon(&self, evidence: &Vec<MessageSlot<ValidatorId>>) -> Vec<ValidatorId> {
        evidence.iter().filter(|slot| slot.round == self.0).map(|slot| slot.validator).collect()
    }
}

// C commits one target only with distinct supermajority certificate authors.
struct CommitQuorum<Sorts> {
    sorts: PhantomData<Sorts>,
    candidate: CandidateId,
    committee: Vec<ValidatorId>,
}
impl<Sorts: CollapseSorts> CollapseSorts for CommitQuorum<Sorts> {
    type History = Sorts::History;
    type Evidence = Sorts::Evidence;
    type Visible = Sorts::Visible;
    type Output = Sorts::Output;
}

impl CollapseRule for CommitQuorum<Certificates> {
    fn collapse(&self, visible: &Vec<ValidatorId>) -> Option<CandidateId> {
        let support = visible.iter().filter(|v| self.committee.contains(v)).collect::<BTreeSet<_>>();
        TwoThirdsQuorum.meets_quorum(support.len() as u128, self.committee.len() as u128).then_some(self.candidate)
    }
}

// A set interpretation forgets repeated certificates by the same author in the same round.
struct CertificateSets(CandidateId);
impl CollapseSorts for CertificateSets {
    type History = History;
    type Evidence = BTreeSet<(u64, u64)>;
    type Visible = BTreeSet<u64>;
    type Output = u64;
}

impl EvidenceExtraction for CertificateSets {
    fn extract_evidence(&self, history: &History) -> BTreeSet<(u64, u64)> {
        history
            .message_slots()
            .into_iter()
            .filter(|slot| {
                history.slot_messages(slot).into_iter().any(|witness| history.mysticeti_certifies(&witness, &self.0))
            })
            .map(|slot| (slot.round, slot.validator.0))
            .collect()
    }
}

impl HorizonProjection for AtRound<CertificateSets> {
    fn project_horizon(&self, evidence: &BTreeSet<(u64, u64)>) -> BTreeSet<u64> {
        evidence.iter().filter(|(round, _)| *round == self.0).map(|(_, author)| *author).collect()
    }
}

impl CollapseRule for CommitQuorum<CertificateSets> {
    fn collapse(&self, visible: &BTreeSet<u64>) -> Option<u64> {
        let support = self.committee.iter().filter(|validator| visible.contains(&validator.0)).count();
        TwoThirdsQuorum.meets_quorum(support as u128, self.committee.len() as u128).then_some(self.candidate.0)
    }
}

struct SetDenotation;
impl Denotation<HistoryDenotation, History, History> for SetDenotation {
    fn denote(&self, _: &HistoryDenotation, history: &History) -> History {
        history.clone()
    }
}
impl Denotation<EvidenceDenotation, Vec<MessageSlot<ValidatorId>>, BTreeSet<(u64, u64)>> for SetDenotation {
    fn denote(&self, _: &EvidenceDenotation, evidence: &Vec<MessageSlot<ValidatorId>>) -> BTreeSet<(u64, u64)> {
        evidence.iter().map(|slot| (slot.round, slot.validator.0)).collect()
    }
}
impl Denotation<VisibleDenotation, Vec<ValidatorId>, BTreeSet<u64>> for SetDenotation {
    fn denote(&self, _: &VisibleDenotation, visible: &Vec<ValidatorId>) -> BTreeSet<u64> {
        visible.iter().map(|validator| validator.0).collect()
    }
}
impl Denotation<OutputDenotation, CandidateId, u64> for SetDenotation {
    fn denote(&self, _: &OutputDenotation, output: &CandidateId) -> u64 {
        output.0
    }
}

#[test]
fn independent_stages_preserve_direct_commit_under_set_denotation() -> Result<()> {
    let mut machine = Machine::new(genesis(), expansion(), ());
    let concrete = CollapsePolicy::new(
        Certificates(CandidateId(0)),
        AtRound(2, PhantomData::<Certificates>),
        CommitQuorum::<Certificates> {
            sorts: PhantomData,
            candidate: CandidateId(0),
            committee: machine.history().validators(),
        },
    );
    let specification = CollapsePolicy::new(
        CertificateSets(CandidateId(0)),
        AtRound(2, PhantomData::<CertificateSets>),
        CommitQuorum::<CertificateSets> {
            sorts: PhantomData,
            candidate: CandidateId(0),
            committee: machine.history().validators(),
        },
    );
    let policy = mysticeti_policy(2);
    // Check each reachable prefix, including absence of certificates and insufficient support.
    for round in 0..3 {
        let parents = if round == 0 { vec![] } else { (0..3).map(|v| (round - 1) * 10 + v).collect() };
        for validator in 0..3 {
            append(&mut machine, round, validator, round * 10 + validator, &parents);
            let history = machine.history();
            SetDenotation.check_realization(&concrete, &specification, history)?;
            let expected = MysticetiDagEvidence::new(history, &policy).match_horizon(&CandidateId(0));
            assert_eq!(concrete.collapse_policy(history), expected);
        }
    }
    assert_eq!(concrete.collapse_policy(machine.history()), Some(CandidateId(0)));
    // More certificates from the same author change the list, but not its set denotation or output.
    append(&mut machine, 2, 0, 200, &[10, 11, 12]);
    SetDenotation.check_realization(&concrete, &specification, machine.history())?;
    assert_eq!(concrete.collapse_policy(machine.history()), Some(CandidateId(0)));
    Ok(())
}

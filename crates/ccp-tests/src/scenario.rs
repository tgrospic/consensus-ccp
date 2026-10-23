use ccp_communication::{ExpansionError, MessageInput, MessageSlot, QuorumAdvancePolicy, quorum_advance_policy};
use ccp_cordial::CordialPolicy;
use ccp_core::Closed;
use ccp_mysticeti::MysticetiPolicy;
use ccp_runtime::{CandidateId, CommunicationExpansion, CommunicationHistory, ObservedSlot, ValidatorId};
use ccp_scenarios::{
    CommunicationExpansionExpect, CommunicationHistoryScenario, CommunicationRetentionExpect, CordialAnchorsExpect,
    CordialCollapseScenario, CordialEvidenceExpect, DenseCollapseScenario, HorizonAgreementExpect,
    MysticetiCertificateExpect, MysticetiCollapseScenario, ObservedDecisions, PositiveEvidenceExpect, Scenario,
    ScenarioHistory, ScheduledCollapse, SlotDecisions,
};

pub(crate) type History = CommunicationHistory<ValidatorId, CandidateId>;
pub(crate) type Input = MessageInput<ValidatorId, CandidateId>;
pub(crate) type Expansion = CommunicationExpansion<QuorumAdvancePolicy>;
type ObservedDecision = (ObservedSlot<ValidatorId, CandidateId>, Option<Closed<CandidateId>>);
type SlotDecision = (MessageSlot<ValidatorId>, Option<Closed<CandidateId>>);

/// Provides the concrete authoring context selected by `ccp_test!`.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct TestScenarios;

/// Retains authored histories until the runner interprets them.
#[derive(Debug, Default)]
pub(crate) struct TestScenario {
    pub(crate) histories: Vec<HistorySpec>,
}

/// Describes one reachable history and the observations required of it.
#[derive(Debug, Default)]
pub(crate) struct HistorySpec {
    pub(crate) transitions: Vec<Transition>,
    pub(crate) expectations: Vec<Expectation>,
}

/// Describes whether one authored input must be admitted.
#[derive(Debug, Clone)]
pub(crate) enum Transition {
    Accept(Input),
    Reject(Input, ExpansionError),
}

/// Describes one semantic observation interpreted after history construction.
#[derive(Debug)]
pub(crate) enum Expectation {
    Retention(Vec<CandidateId>),
    SlotRetention(MessageSlot<ValidatorId>, Vec<CandidateId>),
    Collapse(CollapseExpectation),
    PositiveEvidence {
        policy: CordialPolicy,
        witness: CandidateId,
        candidate: CandidateId,
    },
    CordialEvidence {
        policy: CordialPolicy,
        witness: CandidateId,
        candidate: CandidateId,
        later: CandidateId,
    },
    HorizonAgreement {
        cordial: CordialPolicy,
        mysticeti: MysticetiPolicy,
        observer: CandidateId,
        candidate: CandidateId,
    },
    CordialAnchors {
        policy: CordialPolicy,
        observer: CandidateId,
        first_slot: MessageSlot<ValidatorId>,
        later_slot: MessageSlot<ValidatorId>,
        first: CandidateId,
        later: CandidateId,
    },
    MysticetiCertificate {
        policy: MysticetiPolicy,
        certificate: CandidateId,
        non_certificate: CandidateId,
        anchor: CandidateId,
        candidate: CandidateId,
        expected_link: bool,
    },
    FutureCone {
        choice: Input,
        max_steps: usize,
        expected_messages: Vec<Vec<CandidateId>>,
    },
    ExpansionAndCollapse {
        input: Input,
        expected_messages: Vec<CandidateId>,
    },
}

/// Describes one protocol collapse table without executing it while authoring.
#[derive(Debug)]
pub(crate) enum CollapseExpectation {
    Dense(Vec<ObservedDecision>),
    Scheduled(MysticetiPolicy, Vec<ObservedDecision>),
    ScheduledCordial(CordialPolicy, Vec<ObservedDecision>),
    Cordial(CordialPolicy, Vec<ObservedDecision>),
    Mysticeti(MysticetiPolicy, Vec<SlotDecision>),
}

enum ObservedCollapseKind {
    Dense,
    Scheduled(MysticetiPolicy),
    ScheduledCordial(CordialPolicy),
    Cordial(CordialPolicy),
}

pub(crate) struct ObservedCollapseSpec {
    history: HistorySpec,
    kind: ObservedCollapseKind,
    cases: Vec<ObservedDecision>,
}

pub(crate) struct SlotCollapseSpec {
    history: HistorySpec,
    policy: MysticetiPolicy,
    cases: Vec<SlotDecision>,
}

impl Scenario for TestScenarios {
    type Scenario = TestScenario;
    type History = HistorySpec;

    fn scenario(&self) -> Self::Scenario {
        TestScenario::default()
    }

    fn history(&self) -> Self::History {
        HistorySpec::default()
    }
}

impl ScenarioHistory for TestScenario {
    type History = HistorySpec;

    fn history(mut self, history: Self::History) -> Self {
        self.histories.push(history);
        self
    }
}

impl HistorySpec {
    fn with_expectation(mut self, expectation: Expectation) -> Self {
        self.expectations.push(expectation);
        self
    }
}

pub(crate) fn expansion() -> Expansion {
    CommunicationExpansion::new(quorum_advance_policy())
}

pub(crate) fn genesis() -> History {
    CommunicationHistory::genesis((0..4).map(ValidatorId).collect()).expect("four validators form a committee")
}

pub(crate) fn input(round: u64, validator: u64, candidate: u64, justifications: &[u64]) -> Input {
    MessageInput::new(
        MessageSlot::new(round, ValidatorId(validator)),
        CandidateId(candidate),
        justifications.iter().copied().map(CandidateId).collect(),
    )
}

pub(crate) fn observed(round: u64, validator: u64, observer: u64) -> ObservedSlot<ValidatorId, CandidateId> {
    ObservedSlot::new(MessageSlot::new(round, ValidatorId(validator)), CandidateId(observer))
}

impl CommunicationHistoryScenario for HistorySpec {
    fn message(self, round: u64, validator: u64, candidate: u64, justifications: &[u64]) -> Self {
        let mut history = self;
        history.transitions.push(Transition::Accept(input(round, validator, candidate, justifications)));
        history
    }

    fn reject_insufficient_advance(self, round: u64, validator: u64, candidate: u64, justifications: &[u64]) -> Self {
        self.reject(round, validator, candidate, justifications, ExpansionError::InsufficientAdvanceEvidence)
    }

    fn reject_non_causal(self, round: u64, validator: u64, candidate: u64, justifications: &[u64]) -> Self {
        self.reject(round, validator, candidate, justifications, ExpansionError::NonCausalJustification)
    }

    fn reject_equivocation_bound(self, round: u64, validator: u64, candidate: u64, justifications: &[u64]) -> Self {
        self.reject(round, validator, candidate, justifications, ExpansionError::EquivocationBoundExceeded)
    }
}

impl HistorySpec {
    fn reject(
        mut self,
        round: u64,
        validator: u64,
        candidate: u64,
        justifications: &[u64],
        error: ExpansionError,
    ) -> Self {
        self.transitions.push(Transition::Reject(input(round, validator, candidate, justifications), error));
        self
    }
}

impl CommunicationRetentionExpect for HistorySpec {
    fn retains(self, candidates: impl IntoIterator<Item = u64>) -> Self {
        let messages = candidates.into_iter().map(CandidateId).collect();
        self.with_expectation(Expectation::Retention(messages))
    }

    fn retains_at(self, round: u64, validator: u64, candidates: impl IntoIterator<Item = u64>) -> Self {
        let slot = MessageSlot::new(round, ValidatorId(validator));
        let candidates = candidates.into_iter().map(CandidateId).collect();
        self.with_expectation(Expectation::SlotRetention(slot, candidates))
    }
}

impl DenseCollapseScenario for HistorySpec {
    type Collapse = ObservedCollapseSpec;

    fn dense_collapse(self) -> Self::Collapse {
        ObservedCollapseSpec { history: self, kind: ObservedCollapseKind::Dense, cases: Vec::new() }
    }
}

impl ScheduledCollapse<MysticetiPolicy> for HistorySpec {
    type Collapse = ObservedCollapseSpec;

    fn scheduled_collapse(self, policy: MysticetiPolicy) -> Self::Collapse {
        ObservedCollapseSpec { history: self, kind: ObservedCollapseKind::Scheduled(policy), cases: Vec::new() }
    }
}

impl ScheduledCollapse<CordialPolicy> for HistorySpec {
    type Collapse = ObservedCollapseSpec;

    fn scheduled_collapse(self, policy: CordialPolicy) -> Self::Collapse {
        ObservedCollapseSpec { history: self, kind: ObservedCollapseKind::ScheduledCordial(policy), cases: Vec::new() }
    }
}

impl CordialCollapseScenario<CordialPolicy> for HistorySpec {
    type Collapse = ObservedCollapseSpec;

    fn cordial_collapse(self, policy: CordialPolicy) -> Self::Collapse {
        ObservedCollapseSpec { history: self, kind: ObservedCollapseKind::Cordial(policy), cases: Vec::new() }
    }
}

impl ObservedDecisions<u64, u64, u64> for ObservedCollapseSpec {
    type History = HistorySpec;

    fn commit(mut self, round: u64, validator: u64, observer: u64, candidate: u64) -> Self {
        self.cases.push((observed(round, validator, observer), Some(Closed::Commit(CandidateId(candidate)))));
        self
    }

    fn skip(mut self, round: u64, validator: u64, observer: u64) -> Self {
        self.cases.push((observed(round, validator, observer), Some(Closed::Skip)));
        self
    }

    fn undecided(mut self, round: u64, validator: u64, observer: u64) -> Self {
        self.cases.push((observed(round, validator, observer), None));
        self
    }

    fn not_selected(self, round: u64, validator: u64, observer: u64) -> Self {
        self.undecided(round, validator, observer)
    }

    fn finish(self) -> Self::History {
        let expectation = match self.kind {
            ObservedCollapseKind::Dense => CollapseExpectation::Dense(self.cases),
            ObservedCollapseKind::Scheduled(policy) => CollapseExpectation::Scheduled(policy, self.cases),
            ObservedCollapseKind::ScheduledCordial(policy) => CollapseExpectation::ScheduledCordial(policy, self.cases),
            ObservedCollapseKind::Cordial(policy) => CollapseExpectation::Cordial(policy, self.cases),
        };
        self.history.with_expectation(Expectation::Collapse(expectation))
    }
}

impl MysticetiCollapseScenario<MysticetiPolicy> for HistorySpec {
    type Collapse = SlotCollapseSpec;

    fn mysticeti_collapse(self, policy: MysticetiPolicy) -> Self::Collapse {
        SlotCollapseSpec { history: self, policy, cases: Vec::new() }
    }
}

impl SlotDecisions<u64, u64, u64> for SlotCollapseSpec {
    type History = HistorySpec;

    fn commit(mut self, round: u64, validator: u64, candidate: u64) -> Self {
        self.cases
            .push((MessageSlot::new(round, ValidatorId(validator)), Some(Closed::Commit(CandidateId(candidate)))));
        self
    }

    fn skip(mut self, round: u64, validator: u64) -> Self {
        self.cases.push((MessageSlot::new(round, ValidatorId(validator)), Some(Closed::Skip)));
        self
    }

    fn undecided(mut self, round: u64, validator: u64) -> Self {
        self.cases.push((MessageSlot::new(round, ValidatorId(validator)), None));
        self
    }

    fn finish(self) -> Self::History {
        self.history.with_expectation(Expectation::Collapse(CollapseExpectation::Mysticeti(self.policy, self.cases)))
    }
}

impl PositiveEvidenceExpect<CordialPolicy, u64> for HistorySpec {
    fn expect_positive_evidence(self, policy: CordialPolicy, witness: u64, candidate: u64) -> Self {
        self.with_expectation(Expectation::PositiveEvidence {
            policy,
            witness: CandidateId(witness),
            candidate: CandidateId(candidate),
        })
    }
}

impl CordialEvidenceExpect<CordialPolicy, u64> for HistorySpec {
    fn expect_cordial_evidence(self, policy: CordialPolicy, witness: u64, candidate: u64, later: u64) -> Self {
        self.with_expectation(Expectation::CordialEvidence {
            policy,
            witness: CandidateId(witness),
            candidate: CandidateId(candidate),
            later: CandidateId(later),
        })
    }
}

impl HorizonAgreementExpect<CordialPolicy, MysticetiPolicy, u64> for HistorySpec {
    fn expect_horizon_agreement(
        self,
        cordial: CordialPolicy,
        mysticeti: MysticetiPolicy,
        observer: u64,
        candidate: u64,
    ) -> Self {
        self.with_expectation(Expectation::HorizonAgreement {
            cordial,
            mysticeti,
            observer: CandidateId(observer),
            candidate: CandidateId(candidate),
        })
    }
}

impl CordialAnchorsExpect<CordialPolicy, u64, u64, u64> for HistorySpec {
    fn expect_cordial_anchors(
        self,
        policy: CordialPolicy,
        observer: u64,
        slots: ((u64, u64), (u64, u64)),
        anchors: (u64, u64),
    ) -> Self {
        self.with_expectation(Expectation::CordialAnchors {
            policy,
            observer: CandidateId(observer),
            first_slot: MessageSlot::new(slots.0.0, ValidatorId(slots.0.1)),
            later_slot: MessageSlot::new(slots.1.0, ValidatorId(slots.1.1)),
            first: CandidateId(anchors.0),
            later: CandidateId(anchors.1),
        })
    }
}

impl MysticetiCertificateExpect<MysticetiPolicy, u64> for HistorySpec {
    fn expect_mysticeti_certificate(
        self,
        policy: MysticetiPolicy,
        witnesses: (u64, u64),
        anchor: u64,
        candidate: u64,
        expected_link: bool,
    ) -> Self {
        self.with_expectation(Expectation::MysticetiCertificate {
            policy,
            certificate: CandidateId(witnesses.0),
            non_certificate: CandidateId(witnesses.1),
            anchor: CandidateId(anchor),
            candidate: CandidateId(candidate),
            expected_link,
        })
    }
}

impl CommunicationExpansionExpect for HistorySpec {
    fn expect_message_future(
        self,
        message: (u64, u64, u64),
        justifications: &[u64],
        max_steps: usize,
        expected_messages: impl IntoIterator<Item = Vec<u64>>,
    ) -> Self {
        self.with_expectation(Expectation::FutureCone {
            choice: input(message.0, message.1, message.2, justifications),
            max_steps,
            expected_messages: expected_messages
                .into_iter()
                .map(|messages| messages.into_iter().map(CandidateId).collect())
                .collect(),
        })
    }

    fn expect_message_expansion(
        self,
        message: (u64, u64, u64),
        justifications: &[u64],
        expected_messages: impl IntoIterator<Item = u64>,
    ) -> Self {
        self.with_expectation(Expectation::ExpansionAndCollapse {
            input: input(message.0, message.1, message.2, justifications),
            expected_messages: expected_messages.into_iter().map(CandidateId).collect(),
        })
    }
}

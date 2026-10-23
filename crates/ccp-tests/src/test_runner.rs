use crate::expectations::{
    CommonEvidenceExpectationExt, CordialAnchorExpectationExt, CordialEvidenceExpectationExt,
    HistoryCollapseExpectationExt, HorizonAgreementExpectationExt, MessageCarrierExpectationExt,
    MysticetiCertificateExpectationExt,
};
use crate::law_runner::{ProtocolLaws, check_carrier_laws};
use crate::scenario::{
    CollapseExpectation, Expansion, Expectation, History, HistorySpec, Input as ScenarioInput, TestScenario,
    Transition, expansion, genesis,
};
use anyhow::{Result, anyhow, ensure};
use ccp_communication::{
    ExpansionError, MessageCarrierExt, MessageExpansionLawsExt, MessageHistoryLawsExt, quorum_advance_policy,
};
use ccp_core::{ExpansionChoices, FiniteExpansionExt, HistoryCollapse, HistoryExpansion};
use ccp_mysticeti::MysticetiDagEvidence;
use ccp_runtime::{
    CommunicationExpansion, CordialCollapse, CordialDagView, DenseLeaderCollapse, Machine, MysticetiCollapse,
    ScheduledLeaderCollapse,
};

macro_rules! ccp_test {
    ($name:ident) => {
        #[test]
        fn $name() {
            $crate::scenario::TestScenarios.$name().run().expect("CCP conformance scenario failed");
        }
    };
}

pub(crate) use ccp_test;

#[derive(Debug, Clone)]
struct OneChoice(ScenarioInput);

impl ExpansionChoices<History, ScenarioInput> for OneChoice {
    fn expansion_choices(&self, _history: &History) -> Vec<ScenarioInput> {
        vec![self.0.clone()]
    }
}

#[derive(Debug, Clone, Copy)]
struct CandidateCount;

impl HistoryCollapse<History, (), usize> for CandidateCount {
    fn collapse_history(&self, history: &History, _query: &()) -> Option<usize> {
        Some(history.messages().len())
    }
}

impl HistoryExpansion<History> for CandidateCount {
    type Input = ScenarioInput;
    type Error = ExpansionError;

    fn expand(&self, history: &History, input: &Self::Input) -> Result<History, Self::Error> {
        CommunicationExpansion::new(quorum_advance_policy()).expand(history, input)
    }
}

impl TestScenario {
    pub(crate) fn run(self) -> Result<()> {
        self.histories.into_iter().try_for_each(run_history)
    }
}

fn run_history(spec: HistorySpec) -> Result<()> {
    let expansion = expansion();
    let laws = spec
        .expectations
        .iter()
        .filter_map(|expectation| match expectation {
            Expectation::Collapse(collapse) => Some(ProtocolLaws::of(collapse)),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut machine = Machine::new(genesis(), expansion, ());
    machine.history().check_lawful_message_history(&quorum_advance_policy())?;
    check_history_laws(machine.history(), &laws)?;
    spec.transitions.into_iter().try_for_each(|transition| apply_transition(&mut machine, &laws, transition))?;
    let history = machine.history();
    spec.expectations.into_iter().try_for_each(|expectation| check_expectation(history, &expansion, expectation))
}

fn apply_transition(
    machine: &mut Machine<History, Expansion, ()>,
    laws: &[ProtocolLaws],
    transition: Transition,
) -> Result<()> {
    let before = machine.history().clone();
    let input = match &transition {
        Transition::Accept(input) | Transition::Reject(input, _) => input,
    };
    expansion().check_message_expansion(&before, &quorum_advance_policy(), input)?;
    match transition {
        Transition::Accept(input) => {
            machine.apply(&input).map_err(|error| anyhow!("accepted input was rejected: {error:?}"))?;
            laws.iter().try_for_each(|law| law.check_expansion(&before, machine.history(), &input))?;
        }
        Transition::Reject(input, expected) => {
            let actual = machine.apply(&input);
            ensure!(actual == Err(expected), "rejected input produced {actual:?}, expected {expected:?}");
            ensure!(machine.history() == &before, "rejected input changed the reachable history");
        }
    }
    check_history_laws(machine.history(), laws)
}

fn check_history_laws(history: &History, laws: &[ProtocolLaws]) -> Result<()> {
    check_carrier_laws(history)?;
    laws.iter().try_for_each(|law| law.check_history(history))
}

fn check_expectation(history: &History, expansion: &Expansion, expectation: Expectation) -> Result<()> {
    match expectation {
        Expectation::Retention(messages) => history.check_retention(&messages, None),
        Expectation::SlotRetention(slot, candidates) => history.check_slot_retention(&slot, &candidates),
        Expectation::Collapse(collapse) => check_collapse(history, collapse),
        Expectation::PositiveEvidence { policy, witness, candidate } => {
            history.check_positive_evidence_composition(&policy, &witness, &candidate)
        }
        Expectation::CordialEvidence { policy, witness, candidate, later } => {
            history.check_cordial_evidence(&witness, &candidate, &later, &policy)
        }
        Expectation::HorizonAgreement { cordial, mysticeti, observer, candidate } => {
            let cordial = CordialDagView::new(history, &cordial, &observer);
            let mysticeti = MysticetiDagEvidence::new(history, &mysticeti);
            cordial.check_positive_horizon_agreement(&mysticeti, &candidate)
        }
        Expectation::CordialAnchors { policy, observer, first_slot, later_slot, first, later } => CordialDagView::new(
            history, &policy, &observer,
        )
        .check_cordial_anchor_case(&first_slot, &later_slot, &first, &later),
        Expectation::MysticetiCertificate {
            policy,
            certificate,
            non_certificate,
            anchor,
            candidate,
            expected_link,
        } => history.check_mysticeti_certificate(
            &certificate,
            &non_certificate,
            &anchor,
            &candidate,
            &policy,
            expected_link,
        ),
        Expectation::FutureCone { choice, max_steps, expected_messages } => {
            let actual = expansion
                .expansion_future_cone(history, &OneChoice(choice), max_steps)
                .into_iter()
                .map(|history| history.messages())
                .collect::<Vec<_>>();
            ensure!(actual == expected_messages, "future cone retained {actual:?}, expected {expected_messages:?}");
            Ok(())
        }
        Expectation::ExpansionAndCollapse { input, expected_messages } => {
            history.check_collapse_cases(&CandidateCount, &[((), Some(history.messages().len()))])?;
            let successor =
                expansion.expand(history, &input).map_err(|error| anyhow!("expansion was rejected: {error:?}"))?;
            ensure!(
                successor.messages() == expected_messages,
                "expansion retained {:?}, expected {expected_messages:?}",
                successor.messages()
            );
            Ok(())
        }
    }
}

fn check_collapse(history: &History, expectation: CollapseExpectation) -> Result<()> {
    match expectation {
        CollapseExpectation::Dense(cases) => history.check_collapse_cases(&DenseLeaderCollapse, &cases),
        CollapseExpectation::Scheduled(policy, cases) => {
            history.check_collapse_cases(&ScheduledLeaderCollapse::new(policy), &cases)
        }
        CollapseExpectation::ScheduledCordial(policy, cases) => {
            history.check_collapse_cases(&ScheduledLeaderCollapse::new(policy), &cases)
        }
        CollapseExpectation::Cordial(policy, cases) => {
            history.check_collapse_cases(&CordialCollapse::new(policy), &cases)
        }
        CollapseExpectation::Mysticeti(policy, cases) => {
            history.check_collapse_cases(&MysticetiCollapse::new(policy), &cases)
        }
    }
}

//! Interprets the communication scenario language immediately, without recording a syntax tree.
use crate::expectations::MessageCarrierExpectationExt;
use crate::scenario::{Expansion, History, expansion, genesis};
use anyhow::{Result, anyhow, ensure};
use ccp_communication::{ExpansionError, MessageExpansionLawsExt, MessageInput, MessageSlot, quorum_advance_policy};
use ccp_runtime::{CandidateId, Machine, ValidatorId};
use ccp_scenarios::{
    CommunicationHistoryScenario, CommunicationRetentionExpect, CommunicationScenariosExt, Scenario, ScenarioHistory,
};

struct Direct;
struct DirectHistory(Result<Machine<History, Expansion, ()>>);
struct DirectScenario(Result<()>);

impl Scenario for Direct {
    type Scenario = DirectScenario;
    type History = DirectHistory;

    fn scenario(&self) -> DirectScenario {
        DirectScenario(Ok(()))
    }

    fn history(&self) -> DirectHistory {
        DirectHistory(Ok(Machine::new(genesis(), expansion(), ())))
    }
}

impl ScenarioHistory for DirectScenario {
    type History = DirectHistory;

    fn history(self, history: DirectHistory) -> Self {
        Self(self.0.and(history.0.map(|_| ())))
    }
}

impl DirectHistory {
    fn apply(
        self,
        round: u64,
        validator: u64,
        candidate: u64,
        justifications: &[u64],
        rejection: Option<ExpansionError>,
    ) -> Self {
        Self(self.0.and_then(|mut machine| {
            let before = machine.history().clone();
            let input = MessageInput::new(
                MessageSlot::new(round, ValidatorId(validator)),
                CandidateId(candidate),
                justifications.iter().copied().map(CandidateId).collect(),
            );
            expansion().check_message_expansion(&before, &quorum_advance_policy(), &input)?;
            let result = machine.apply(&input);
            match rejection {
                Some(error) => {
                    ensure!(result == Err(error), "expected rejection {error:?}, got {result:?}");
                    ensure!(machine.history() == &before, "rejected input changed history");
                }
                None => result.map_err(|error| anyhow!("accepted input was rejected: {error:?}"))?,
            }
            Ok(machine)
        }))
    }
}

impl CommunicationHistoryScenario for DirectHistory {
    fn message(self, round: u64, validator: u64, candidate: u64, justifications: &[u64]) -> Self {
        self.apply(round, validator, candidate, justifications, None)
    }

    fn reject_insufficient_advance(self, round: u64, validator: u64, candidate: u64, justifications: &[u64]) -> Self {
        self.apply(round, validator, candidate, justifications, Some(ExpansionError::InsufficientAdvanceEvidence))
    }

    fn reject_non_causal(self, round: u64, validator: u64, candidate: u64, justifications: &[u64]) -> Self {
        self.apply(round, validator, candidate, justifications, Some(ExpansionError::NonCausalJustification))
    }

    fn reject_equivocation_bound(self, round: u64, validator: u64, candidate: u64, justifications: &[u64]) -> Self {
        self.apply(round, validator, candidate, justifications, Some(ExpansionError::EquivocationBoundExceeded))
    }
}

impl CommunicationRetentionExpect for DirectHistory {
    fn retains(self, candidates: impl IntoIterator<Item = u64>) -> Self {
        Self(self.0.and_then(|machine| {
            let candidates = candidates.into_iter().map(CandidateId).collect::<Vec<_>>();
            machine.history().check_retention(&candidates, None)?;
            Ok(machine)
        }))
    }

    fn retains_at(self, round: u64, validator: u64, candidates: impl IntoIterator<Item = u64>) -> Self {
        Self(self.0.and_then(|machine| {
            let candidates = candidates.into_iter().map(CandidateId).collect::<Vec<_>>();
            machine.history().check_slot_retention(&MessageSlot::new(round, ValidatorId(validator)), &candidates)?;
            Ok(machine)
        }))
    }
}

// No collapse, certificate, or repair capabilities are implemented by this interpreter.
// These are the very same terms also interpreted by the recorded scenario runner.
#[test]
fn communication_scenarios_also_have_an_immediate_interpretation() -> Result<()> {
    Direct.rejected_input_preserves_the_reachable_history().0?;
    Direct.every_non_genesis_message_carries_previous_round_quorum().0?;
    Direct.carrier_retains_one_byzantine_equivocator_but_rejects_a_second().0
}

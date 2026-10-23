//! Authors scenarios over the communication carrier's expansion and retention.
use crate::{
    CommunicationHistoryScenario, CommunicationHistoryScenarioExt, CommunicationRetentionExpect, Scenario,
    ScenarioHistory,
};
use alux_ext::ext;

/// Authors reusable scenarios over communication-history authoring capabilities.
#[ext(name = CommunicationScenariosExt)]
pub impl<This, Histories, History> This
where
    This: Scenario<Scenario = Histories, History = History>,
    Histories: ScenarioHistory<History = History>,
    History: CommunicationHistoryScenario + CommunicationRetentionExpect,
{
    /// Checks rejected input preserves the reachable history.
    fn rejected_input_preserves_the_reachable_history(&self) -> This::Scenario {
        self.scenario()
            .history(self.history().message(0, 0, 0, &[]).reject_insufficient_advance(1, 1, 10, &[0]).retains([0]))
    }

    /// Checks every non genesis message carries previous round quorum.
    fn every_non_genesis_message_carries_previous_round_quorum(&self) -> This::Scenario {
        self.scenario().history(
            self.history()
                .candidate_round(0, [0, 1, 2], &[])
                .message(1, 0, 10, &[0, 1, 2])
                .reject_non_causal(1, 1, 11, &[0, 1, 2, 10])
                .retains([0, 1, 2, 10]),
        )
    }

    /// Checks carrier retains one byzantine equivocator but rejects a second.
    fn carrier_retains_one_byzantine_equivocator_but_rejects_a_second(&self) -> This::Scenario {
        self.scenario().history(
            self.history()
                .candidate_round(0, [0, 1, 2], &[])
                .message(0, 0, 100, &[])
                .reject_equivocation_bound(0, 1, 101, &[])
                .retains([0, 100, 1, 2])
                .retains_at(0, 0, [0, 100]),
        )
    }
}

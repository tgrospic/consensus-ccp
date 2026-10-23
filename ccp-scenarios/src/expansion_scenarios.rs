//! Authors scenarios that compare finite exploration with live communication expansion.
use crate::{CommunicationExpansionExpect, Scenario, ScenarioHistory};
use alux_ext::ext;

/// Authors reusable scenarios over communication-history authoring capabilities.
#[ext(name = ExpansionScenariosExt)]
pub impl<This, Histories, History> This
where
    This: Scenario<Scenario = Histories, History = History>,
    Histories: ScenarioHistory<History = History>,
    History: CommunicationExpansionExpect,
{
    /// Checks finite exploration executes the same constructive expansion.
    fn finite_exploration_executes_the_same_constructive_expansion(&self) -> This::Scenario {
        self.scenario().history(self.history().expect_message_future((0, 0, 0), &[], 1, [Vec::new(), vec![0]]))
    }

    /// Checks expansion and collapse are distinct meanings over the same history.
    fn expansion_and_collapse_are_distinct_meanings_over_the_same_history(&self) -> This::Scenario {
        self.scenario().history(self.history().expect_message_expansion((0, 0, 0), &[], [0]))
    }
}

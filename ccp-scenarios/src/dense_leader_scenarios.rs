//! Authors dense-leader collapse scenarios.
use crate::{
    CommunicationHistoryScenario, CommunicationHistoryScenarioExt, DenseCollapseScenario, ObservedDecisions, Scenario,
    ScenarioHistory,
};
use alux_ext::ext;

/// Authors reusable scenarios over the capabilities required by this protocol.
#[ext(name = DenseLeaderScenariosExt)]
pub impl<This, Histories, History> This
where
    This: Scenario<Scenario = Histories, History = History>,
    Histories: ScenarioHistory<History = History>,
    History: CommunicationHistoryScenario,
{
    /// Checks dense leader collapse is derived from the observers reachable history.
    fn dense_leader_collapse_is_derived_from_the_observers_reachable_history<Dense>(&self) -> This::Scenario
    where
        History: DenseCollapseScenario<Collapse = Dense>,
        Dense: ObservedDecisions<u64, u64, u64, History = History>,
    {
        self.scenario().history(
            self.history()
                .fully_connected([3, 3, 3, 3, 1])
                .dense_collapse()
                .commit(0, 0, 40, 0)
                .skip(0, 3, 40)
                .commit(2, 0, 40, 20)
                .undecided(3, 0, 40)
                .undecided(4, 0, 40)
                .finish(),
        )
    }
}

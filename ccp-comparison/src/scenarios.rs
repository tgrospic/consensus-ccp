use alux_ext::ext;
use ccp_cordial::{CordialPolicy, cordial_policy};
use ccp_mysticeti::{MysticetiPolicy, mysticeti_policy};
use ccp_scenarios::{
    CommunicationHistoryScenario, CommunicationHistoryScenarioExt, HorizonAgreementExpect, ObservedDecisions, Scenario,
    ScenarioHistory, ScheduledCollapse,
};
/// Authors comparisons between independently specified protocol families.
#[ext(name = ComparisonScenariosExt)]
pub impl<This, Histories, History> This
where
    This: Scenario<Scenario = Histories, History = History>,
    Histories: ScenarioHistory<History = History>,
    History: CommunicationHistoryScenario,
{
    /// Checks cordial and mysticeti share the positive horizon match.
    fn cordial_and_mysticeti_share_the_positive_horizon_match(&self) -> This::Scenario
    where
        History: HorizonAgreementExpect<CordialPolicy, MysticetiPolicy, u64>,
    {
        self.scenario().history(self.history().fully_connected([3, 3, 3, 3, 3, 3, 1]).expect_horizon_agreement(
            cordial_policy(0, 3),
            mysticeti_policy(2),
            60,
            0,
        ))
    }
    /// Checks scheduled dense projection respects round robin and wave leader slots.
    fn scheduled_dense_projection_respects_round_robin_and_wave_leader_slots<Scheduled, Wave>(&self) -> This::Scenario
    where
        History: ScheduledCollapse<MysticetiPolicy, Collapse = Scheduled>
            + ScheduledCollapse<CordialPolicy, Collapse = Wave>,
        Scheduled: ObservedDecisions<u64, u64, u64, History = History>,
        Wave: ObservedDecisions<u64, u64, u64, History = History>,
    {
        self.scenario().history(
            self.history()
                .fully_connected([3, 3, 3, 3, 1])
                .scheduled_collapse(mysticeti_policy(2))
                .commit(0, 0, 40, 0)
                .not_selected(0, 1, 40)
                .commit(1, 1, 40, 11)
                .commit(2, 2, 40, 22)
                .finish()
                .scheduled_collapse(cordial_policy(0, 3))
                .commit(0, 0, 40, 0)
                .not_selected(1, 1, 40)
                .undecided(3, 1, 40)
                .finish(),
        )
    }
}

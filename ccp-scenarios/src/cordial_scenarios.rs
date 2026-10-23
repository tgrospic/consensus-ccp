//! Authors Cordial collapse, evidence, and finalized-anchor scenarios.
use crate::{
    CommunicationHistoryScenario, CommunicationHistoryScenarioExt, CordialAnchorsExpect, CordialCollapseScenario,
    CordialEvidenceExpect, ObservedDecisions, Scenario, ScenarioHistory,
};
use alux_ext::ext;
use ccp_cordial::{CordialPolicy, cordial_policy};

/// Authors reusable scenarios over the capabilities required by this protocol.
#[ext(name = CordialScenariosExt)]
pub impl<This, Histories, History> This
where
    This: Scenario<Scenario = Histories, History = History>,
    Histories: ScenarioHistory<History = History>,
    History: CommunicationHistoryScenario,
{
    /// Checks cordial vocabulary names ratification and super ratification.
    fn cordial_vocabulary_names_ratification_and_super_ratification(&self) -> This::Scenario
    where
        History: CordialEvidenceExpect<CordialPolicy, u64>,
    {
        self.scenario().history(self.history().fully_connected([3, 3, 3, 3, 3, 3, 1]).expect_cordial_evidence(
            cordial_policy(0, 3),
            20,
            0,
            31,
        ))
    }

    /// Checks cordial finalized leaders form the complete ratified predecessor chain.
    fn cordial_finalized_leaders_form_the_complete_ratified_predecessor_chain(&self) -> This::Scenario
    where
        History: CordialAnchorsExpect<CordialPolicy, u64, u64, u64>,
    {
        self.scenario().history(self.history().fully_connected([3, 3, 3, 3, 3, 3, 1]).expect_cordial_anchors(
            cordial_policy(0, 3),
            60,
            ((0, 0), (3, 1)),
            (0, 31),
        ))
    }

    /// Checks cordial runtime commits exactly the finalized wave leaders.
    fn cordial_runtime_commits_exactly_the_finalized_wave_leaders<Collapse>(&self) -> This::Scenario
    where
        History: CordialCollapseScenario<CordialPolicy, Collapse = Collapse>,
        Collapse: ObservedDecisions<u64, u64, u64, History = History>,
    {
        self.scenario().history(
            self.history()
                .fully_connected([3, 3, 3, 3, 3, 3, 1])
                .cordial_collapse(cordial_policy(0, 3))
                .commit(0, 0, 60, 0)
                .commit(3, 1, 60, 31)
                .finish(),
        )
    }
}

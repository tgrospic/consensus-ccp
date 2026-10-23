//! Authors Mysticeti collapse, certificate, and repair scenarios.
use crate::{
    CommunicationHistoryScenario, CommunicationHistoryScenarioExt, MysticetiCertificateExpect,
    MysticetiCollapseScenario, MysticetiRepairHistoryScenarioExt, Scenario, ScenarioHistory, SlotDecisions,
};
use alux_ext::ext;
use ccp_mysticeti::{MysticetiPolicy, mysticeti_policy};

/// Authors reusable scenarios over the capabilities required by this protocol.
#[ext(name = MysticetiScenariosExt)]
pub impl<This, Histories, History, Collapse> This
where
    This: Scenario<Scenario = Histories, History = History>,
    Histories: ScenarioHistory<History = History>,
    History: CommunicationHistoryScenario + MysticetiCollapseScenario<MysticetiPolicy, Collapse = Collapse>,
    Collapse: SlotDecisions<u64, u64, u64, History = History>,
{
    /// Checks mysticeti direct commit is derived from reachable messages.
    fn mysticeti_direct_commit_is_derived_from_reachable_messages(&self) -> This::Scenario {
        self.scenario().history(
            self.history()
                .candidate_round(0, [0, 1, 2, 3], &[])
                .candidate_round(1, [10, 11, 12, 13], &[0, 1, 2])
                .candidate_round(2, [20, 21, 22], &[10, 11, 12])
                .mysticeti_collapse(mysticeti_policy(2))
                .commit(0, 0, 0)
                .finish(),
        )
    }

    /// Checks mysticeti indirect repair requires a visible decision round certificate.
    fn mysticeti_indirect_repair_requires_a_visible_decision_round_certificate(&self) -> This::Scenario
    where
        History: MysticetiCertificateExpect<MysticetiPolicy, u64>,
    {
        let policy = mysticeti_policy(2);
        let linked = self
            .history()
            .repair_history_observing_certificate()
            .expect_mysticeti_certificate(policy.clone(), (20, 21), 40, 0, true)
            .mysticeti_collapse(policy.clone())
            .commit(0, 0, 0)
            .finish();
        let unlinked = self
            .history()
            .repair_history_without_certificate()
            .expect_mysticeti_certificate(policy.clone(), (20, 21), 40, 0, false)
            .mysticeti_collapse(policy)
            .skip(0, 0)
            .finish();
        self.scenario().history(linked).history(unlinked)
    }
}

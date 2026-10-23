//! Authors positive-evidence scenarios under the Cordial policy.
use crate::{
    CommunicationHistoryScenario, CommunicationHistoryScenarioExt, PositiveEvidenceExpect, Scenario, ScenarioHistory,
};
use alux_ext::ext;
use ccp_cordial::{CordialPolicy, cordial_policy};

/// Authors reusable scenarios over the capabilities required by this protocol.
#[ext(name = EvidenceScenariosExt)]
pub impl<This, Histories, History> This
where
    This: Scenario<Scenario = Histories, History = History>,
    Histories: ScenarioHistory<History = History>,
    History: CommunicationHistoryScenario + PositiveEvidenceExpect<CordialPolicy, u64>,
{
    /// Checks common evidence composition matches then fires its collapse continuation.
    fn common_evidence_composition_matches_then_fires_its_collapse_continuation(&self) -> This::Scenario {
        self.scenario().history(self.history().fully_connected([3, 3, 3]).expect_positive_evidence(
            cordial_policy(0, 3),
            20,
            0,
        ))
    }
}

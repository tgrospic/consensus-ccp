//! Defines the Mysticeti scenario-authoring vocabulary and its repair-history fixture.
use crate::{CommunicationHistoryScenario, CommunicationHistoryScenarioExt};
use alux_ext::ext;

#[ext(name = MysticetiRepairHistoryScenarioExt)]
/// Derives the paired Mysticeti repair histories from the primitive message language.
pub impl<This> This
where
    This: CommunicationHistoryScenario,
{
    /// Builds a repair history whose later anchor observes the decision certificate.
    fn repair_history_observing_certificate(self) -> Self {
        self.mysticeti_repair_history_from(&[20, 21, 22])
    }

    /// Builds a repair history whose later anchor does not observe the decision certificate.
    fn repair_history_without_certificate(self) -> Self {
        self.mysticeti_repair_history_from(&[21, 22, 23])
    }

    /// Builds the shared repair fixture with explicit round-three justifications.
    fn mysticeti_repair_history_from(self, round_three: &[u64]) -> Self {
        let history = self.candidate_round(0, [0, 1, 2, 3], &[]);
        let history = (0..4).fold(history, |history, validator| {
            let justifications = if validator < 3 { &[0, 1, 2][..] } else { &[1, 2, 3][..] };
            history.message(1, validator, 10 + validator, justifications)
        });
        let history = [(0, &[10, 11, 12][..]), (1, &[10, 11, 13][..]), (2, &[10, 12, 13][..]), (3, &[11, 12, 13][..])]
            .into_iter()
            .fold(history, |history, (validator, justifications)| {
                history.message(2, validator, 20 + validator, justifications)
            });
        history
            .candidate_round(3, [30, 31, 32], round_three)
            .candidate_round(4, [40, 41, 42], &[30, 31, 32])
            .candidate_round(5, [50, 51, 52], &[40, 41, 42])
            .candidate_round(6, [60, 61, 62], &[50, 51, 52])
    }
}

/// Starts Mysticeti collapse authoring.
pub trait MysticetiCollapseScenario<Policy> {
    /// Represents the Mysticeti-collapse expectation language.
    type Collapse;

    /// Selects Mysticeti collapse under `policy`.
    fn mysticeti_collapse(self, policy: Policy) -> Self::Collapse;
}

/// Describes Mysticeti certificate and certified-link observations.
pub trait MysticetiCertificateExpect<Policy, C> {
    /// Expects certificate formation and its link visibility classification.
    #[must_use]
    fn expect_mysticeti_certificate(
        self,
        policy: Policy,
        witnesses: (C, C),
        anchor: C,
        candidate: C,
        expected_link: bool,
    ) -> Self;
}

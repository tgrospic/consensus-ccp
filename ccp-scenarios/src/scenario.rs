use alux_ext::ext;

/// Provides fresh scenario and history specifications.
pub trait Scenario {
    /// Represents one authored scenario.
    type Scenario;
    /// Represents one authored history.
    type History;

    /// Starts an empty scenario specification.
    fn scenario(&self) -> Self::Scenario;

    /// Starts an empty history specification.
    fn history(&self) -> Self::History;
}

/// Describes scenarios composed from independently authored histories.
pub trait ScenarioHistory {
    /// Represents one authored history.
    type History;

    /// Appends a history specification to the scenario.
    #[must_use]
    fn history(self, history: Self::History) -> Self;
}

/// Describes authored communication histories using scenario-level identities.
pub trait CommunicationHistoryScenario {
    /// Appends one admissible message.
    #[must_use]
    fn message(self, round: u64, validator: u64, candidate: u64, justifications: &[u64]) -> Self;

    /// Appends a message rejected for insufficient advance evidence.
    #[must_use]
    fn reject_insufficient_advance(self, round: u64, validator: u64, candidate: u64, justifications: &[u64]) -> Self;

    /// Appends a message rejected for a non-causal justification.
    #[must_use]
    fn reject_non_causal(self, round: u64, validator: u64, candidate: u64, justifications: &[u64]) -> Self;

    /// Appends a message rejected because it exceeds the equivocation bound.
    #[must_use]
    fn reject_equivocation_bound(self, round: u64, validator: u64, candidate: u64, justifications: &[u64]) -> Self;
}

/// Derives common communication-history fixtures from the primitive message language.
#[ext(name = CommunicationHistoryScenarioExt)]
pub impl<This> This
where
    This: CommunicationHistoryScenario,
{
    /// Appends one message from each enumerated validator in a round.
    fn candidate_round<const N: usize>(self, round: u64, candidates: [u64; N], justifications: &[u64]) -> Self {
        candidates.into_iter().enumerate().fold(self, |history, (validator, candidate)| {
            history.message(round, validator as u64, candidate, justifications)
        })
    }

    /// Appends fully connected rounds with the requested production counts.
    fn fully_connected(self, production: impl IntoIterator<Item = usize>) -> Self {
        production
            .into_iter()
            .enumerate()
            .fold((self, Vec::new()), |(history, previous), (round, count)| {
                let candidates = (0..count).map(|validator| round as u64 * 10 + validator as u64).collect::<Vec<_>>();
                let history =
                    candidates.iter().copied().enumerate().fold(history, |history, (validator, candidate)| {
                        history.message(round as u64, validator as u64, candidate, &previous)
                    });
                (history, candidates)
            })
            .0
    }
}

/// Describes retained communication using scenario-level identities.
pub trait CommunicationRetentionExpect {
    /// Expects the complete retained candidate sequence.
    #[must_use]
    fn retains(self, candidates: impl IntoIterator<Item = u64>) -> Self;

    /// Expects every retained candidate at one validator slot.
    #[must_use]
    fn retains_at(self, round: u64, validator: u64, candidates: impl IntoIterator<Item = u64>) -> Self;
}

/// Starts scheduled-leader collapse authoring.
pub trait ScheduledCollapse<Policy> {
    /// Represents the scheduled-collapse expectation language.
    type Collapse;

    /// Selects scheduled-leader collapse under `policy`.
    fn scheduled_collapse(self, policy: Policy) -> Self::Collapse;
}

/// Describes observer-relative collapse decisions without fixing their representation.
pub trait ObservedDecisions<R, V, C> {
    /// Represents the surrounding history specification.
    type History;

    /// Expects an observed slot to commit `candidate`.
    #[must_use]
    fn commit(self, round: R, validator: V, observer: C, candidate: C) -> Self;

    /// Expects an observed slot to skip.
    #[must_use]
    fn skip(self, round: R, validator: V, observer: C) -> Self;

    /// Expects a selected slot to remain undecided.
    #[must_use]
    fn undecided(self, round: R, validator: V, observer: C) -> Self;

    /// Expects a slot to lie outside the selected leader domain.
    #[must_use]
    fn not_selected(self, round: R, validator: V, observer: C) -> Self;

    /// Returns to the surrounding history specification.
    #[must_use]
    fn finish(self) -> Self::History;
}

/// Describes slot-relative collapse decisions without fixing their representation.
pub trait SlotDecisions<R, V, C> {
    /// Represents the surrounding history specification.
    type History;

    /// Expects a slot to commit `candidate`.
    #[must_use]
    fn commit(self, round: R, validator: V, candidate: C) -> Self;

    /// Expects a slot to skip.
    #[must_use]
    fn skip(self, round: R, validator: V) -> Self;

    /// Expects a selected slot to remain undecided.
    #[must_use]
    fn undecided(self, round: R, validator: V) -> Self;

    /// Returns to the surrounding history specification.
    #[must_use]
    fn finish(self) -> Self::History;
}

/// Describes one positive composition of shared quorum evidence.
pub trait PositiveEvidenceExpect<Policy, C> {
    /// Expects `witness` to match and commit `candidate` under `policy`.
    #[must_use]
    fn expect_positive_evidence(self, policy: Policy, witness: C, candidate: C) -> Self;
}

/// Describes agreement between two positive horizon interpretations.
pub trait HorizonAgreementExpect<LeftPolicy, RightPolicy, C> {
    /// Expects both policies to select the same candidate horizon.
    #[must_use]
    fn expect_horizon_agreement(self, left: LeftPolicy, right: RightPolicy, observer: C, candidate: C) -> Self;
}

/// Describes finite expansion observations using scenario-level identities.
pub trait CommunicationExpansionExpect {
    /// Expects the bounded cone generated by one possible message.
    #[must_use]
    fn expect_message_future(
        self,
        message: (u64, u64, u64),
        justifications: &[u64],
        max_steps: usize,
        expected_messages: impl IntoIterator<Item = Vec<u64>>,
    ) -> Self;

    /// Expects one message expansion while preserving the current collapse observation.
    #[must_use]
    fn expect_message_expansion(
        self,
        message: (u64, u64, u64),
        justifications: &[u64],
        expected_messages: impl IntoIterator<Item = u64>,
    ) -> Self;
}

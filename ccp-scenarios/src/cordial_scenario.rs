//! Defines the Cordial scenario-authoring vocabulary.

/// Starts Cordial collapse authoring.
pub trait CordialCollapseScenario<Policy> {
    /// Represents the Cordial-collapse expectation language.
    type Collapse;

    /// Selects Cordial collapse under `policy`.
    fn cordial_collapse(self, policy: Policy) -> Self::Collapse;
}

/// Describes Cordial ratification evidence observations.
pub trait CordialEvidenceExpect<Policy, C> {
    /// Expects ratification and later super-ratification witnesses.
    #[must_use]
    fn expect_cordial_evidence(self, policy: Policy, witness: C, candidate: C, later: C) -> Self;
}

/// Describes Cordial finalized-anchor observations.
pub trait CordialAnchorsExpect<Policy, R, V, C> {
    /// Expects two final leaders and their predecessor relationship.
    #[must_use]
    fn expect_cordial_anchors(self, policy: Policy, observer: C, slots: ((R, V), (R, V)), anchors: (C, C)) -> Self;
}

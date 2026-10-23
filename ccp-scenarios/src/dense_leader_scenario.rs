//! Defines the dense-leader scenario-authoring vocabulary.

/// Starts dense-leader collapse authoring.
pub trait DenseCollapseScenario {
    /// Represents the dense-collapse expectation language.
    type Collapse;

    /// Selects dense-leader collapse for subsequent observations.
    fn dense_collapse(self) -> Self::Collapse;
}

/// Names candidate positions and proposals without prescribing operations.
pub trait CandidateSorts {
    /// Ordered location at which candidates may occur.
    type Slot;
    /// Proposal that may be selected by consensus.
    type Candidate;
}

/// Names the four carriers of a collapse-policy interpretation without prescribing operations.
pub trait CollapseSorts {
    /// Represents communication history.
    type History;
    /// Represents decision-grade evidence extracted from history.
    type Evidence;
    /// Represents evidence visible inside the chosen horizon.
    type Visible;
    /// Represents the terminal observation.
    type Output;
}

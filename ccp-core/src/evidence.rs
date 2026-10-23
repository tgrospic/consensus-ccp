/// Relates an evidence witness to the candidate it supports.
pub trait EvidenceRelation<W, T> {
    /// Reports whether `witness` supplies this relation for `target`.
    fn relates(&self, witness: &W, target: &T) -> bool;
}

/// Matches a query against evidence admitted by a horizon and returns its witness.
pub trait HorizonMatch<Q, W> {
    /// Returns the evidence witness exactly when the visible horizon matches `query`.
    fn match_horizon(&self, query: &Q) -> Option<W>;
}

/// Exposes whether evidence matches now and at an explicit horizon boundary.
pub trait HorizonEvidence<S, T> {
    /// Reports whether the current evidence matches `target`.
    fn current_matches(&self, target: &T) -> bool;

    /// Reports whether the evidence cut ending at `boundary` matches `target`.
    fn matches_at(&self, target: &T, boundary: &S) -> bool;
}

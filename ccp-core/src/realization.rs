//! Defines evidence extraction, horizon matching, collapse, and realization.
use crate::CollapseSorts;
use crate::{Closed, HorizonMatch};
use alux_ext::ext;
use derive_new::new as New;

/// Extracts decision-grade evidence using the interpretation's shared sorts.
pub trait EvidenceExtraction: CollapseSorts {
    /// Extracts decision-grade evidence from `history`.
    fn extract_evidence(&self, history: &Self::History) -> Self::Evidence;
}

/// Restricts evidence using the interpretation's shared sorts.
pub trait HorizonProjection: CollapseSorts {
    /// Projects `evidence` into the visible horizon.
    fn project_horizon(&self, evidence: &Self::Evidence) -> Self::Visible;
}

/// Partially collapses visible evidence using the interpretation's shared sorts.
pub trait CollapseRule: CollapseSorts {
    /// Returns an output exactly when the visible evidence determines one.
    fn collapse(&self, visible: &Self::Visible) -> Option<Self::Output>;
}

/// Composes independent stages whose shared sorts agree at their boundaries.
#[derive(Debug, Clone, Copy, New)]
pub struct CollapsePolicy<Legitimacy, Horizon, Collapse> {
    legitimacy: Legitimacy,
    horizon: Horizon,
    collapse: Collapse,
}

impl<Legitimacy, Horizon, Collapse> CollapseSorts for CollapsePolicy<Legitimacy, Horizon, Collapse>
where
    Legitimacy: CollapseSorts,
    Horizon: CollapseSorts,
    Collapse: CollapseSorts,
{
    type History = Legitimacy::History;
    type Evidence = Legitimacy::Evidence;
    type Visible = Horizon::Visible;
    type Output = Collapse::Output;
}

impl<Legitimacy, Horizon, Collapse> EvidenceExtraction for CollapsePolicy<Legitimacy, Horizon, Collapse>
where
    Legitimacy: EvidenceExtraction,
    Horizon: CollapseSorts,
    Collapse: CollapseSorts,
{
    fn extract_evidence(&self, history: &Self::History) -> Self::Evidence {
        self.legitimacy.extract_evidence(history)
    }
}

impl<Legitimacy, Horizon, Collapse> HorizonProjection for CollapsePolicy<Legitimacy, Horizon, Collapse>
where
    Legitimacy: CollapseSorts,
    Horizon: HorizonProjection<Evidence = Legitimacy::Evidence>,
    Collapse: CollapseSorts,
{
    fn project_horizon(&self, evidence: &Self::Evidence) -> Self::Visible {
        self.horizon.project_horizon(evidence)
    }
}

impl<Legitimacy, Horizon, Collapse> CollapseRule for CollapsePolicy<Legitimacy, Horizon, Collapse>
where
    Legitimacy: CollapseSorts,
    Horizon: CollapseSorts,
    Collapse: CollapseRule<Visible = Horizon::Visible>,
{
    fn collapse(&self, visible: &Self::Visible) -> Option<Self::Output> {
        self.collapse.collapse(visible)
    }
}

/// Continues a successful evidence match into a collapsed output.
pub trait MatchCollapse<W, P> {
    /// Produces the collapsed output denoted by `witness`.
    fn collapse_match(&self, witness: &W) -> P;
}

/// Commits a matched candidate without adding protocol-specific repair.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct CommitMatch;

impl<C> MatchCollapse<C, Closed<C>> for CommitMatch
where
    C: Clone,
{
    fn collapse_match(&self, witness: &C) -> Closed<C> {
        Closed::Commit(witness.clone())
    }
}

/// Composes horizon matching with its collapse continuation.
#[ext(name = MatchedCollapseExt)]
pub impl<This, Q, W> This
where
    This: HorizonMatch<Q, W>,
{
    /// Fires the collapse continuation exactly when the horizon matches.
    fn collapse_when_matched<Collapse, P>(&self, query: &Q, collapse: &Collapse) -> Option<P>
    where
        Collapse: MatchCollapse<W, P>,
    {
        self.match_horizon(query).map(|witness| collapse.collapse_match(&witness))
    }
}

/// Derives the decision map `D = C ∘ R ∘ L`.
#[ext(name = CollapsePolicyExt)]
pub impl<This> This
where
    This: EvidenceExtraction + HorizonProjection + CollapseRule,
{
    /// Applies evidence extraction, horizon projection, and partial collapse.
    fn collapse_policy(&self, history: &This::History) -> Option<This::Output> {
        let evidence = self.extract_evidence(history);
        let visible = self.project_horizon(&evidence);
        self.collapse(&visible)
    }
}

/// Identifies denotation of concrete communication histories.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct HistoryDenotation;

/// Identifies denotation of concrete decision-grade evidence.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct EvidenceDenotation;

/// Identifies denotation of concrete visible evidence.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct VisibleDenotation;

/// Identifies denotation of concrete terminal outputs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct OutputDenotation;

/// Maps one concrete carrier into its specified denotation.
pub trait Denotation<K, I, O> {
    /// Denotes `input` at the meaning identified by `kind`.
    fn denote(&self, kind: &K, input: &I) -> O;
}

//! Defines the state-owning machine boundary.
use ccp_core::{HistoryCollapse, HistoryExpansion};
use derive_new::new as New;

/// Owns one reachable history together with its expansion and collapse meanings.
///
/// The history is readable but cannot be replaced. [`Machine::apply`] is the only state-changing
/// operation and delegates every transition to [`HistoryExpansion`].
#[derive(Debug, Clone, New)]
pub struct Machine<H, Expansion, Collapse> {
    history: H,
    expansion: Expansion,
    collapse: Collapse,
}

impl<H, Expansion, Collapse> Machine<H, Expansion, Collapse> {
    /// Returns the current reachable history.
    pub const fn history(&self) -> &H {
        &self.history
    }

    /// Executes one input through the machine's expansion meaning.
    ///
    /// # Errors
    ///
    /// Returns the expansion interpreter's rejection without changing machine history.
    pub fn apply(&mut self, input: &Expansion::Input) -> Result<(), Expansion::Error>
    where
        Expansion: HistoryExpansion<H>,
    {
        self.history = self.expansion.expand(&self.history, input)?;
        Ok(())
    }

    /// Interprets one query over the current reachable history.
    pub fn collapse<Q, P>(&self, query: &Q) -> Option<P>
    where
        Collapse: HistoryCollapse<H, Q, P>,
    {
        self.collapse.collapse_history(&self.history, query)
    }
}

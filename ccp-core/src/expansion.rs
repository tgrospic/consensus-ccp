//! Defines constructive history expansion and history-relative collapse.

use alux_ext::ext;
use std::collections::VecDeque;

/// Defines the only lawful transition from one history to its successor.
///
/// Inputs are syntax offered to the semantic machine. Implementations validate that syntax and
/// return a new carrier value without mutating the supplied history.
pub trait HistoryExpansion<H> {
    /// Identifies one proposed history-changing input.
    type Input;
    /// Describes why an input cannot extend the supplied history.
    type Error;

    /// Interprets `input` as one lawful history expansion.
    ///
    /// # Errors
    ///
    /// Returns the implementation's typed rejection when `input` is not lawful at `history`.
    fn expand(&self, history: &H, input: &Self::Input) -> Result<H, Self::Error>;
}

/// Enumerates the inputs considered by one finite expansion analysis.
pub trait ExpansionChoices<H, I> {
    /// Returns the possible next inputs at `history`.
    fn expansion_choices(&self, history: &H) -> Vec<I>;
}

/// Partially collapses one reachable history into an observation selected by `query`.
pub trait HistoryCollapse<H, Q, P> {
    /// Returns the observation determined by `history`, if any.
    fn collapse_history(&self, history: &H, query: &Q) -> Option<P>;
}

/// Derives bounded future cones by executing the same expansion used by the live machine.
#[ext(name = FiniteExpansionExt)]
pub impl<This, H> This
where
    This: HistoryExpansion<H>,
    H: Clone + Eq,
{
    /// Returns every history reachable in at most `max_steps` lawful expansions.
    fn expansion_future_cone<Choices>(&self, initial: &H, choices: &Choices, max_steps: usize) -> Vec<H>
    where
        Choices: ExpansionChoices<H, This::Input>,
    {
        let mut histories = vec![initial.clone()];
        let mut pending = VecDeque::from([(initial.clone(), 0)]);

        while let Some((history, steps)) = pending.pop_front() {
            if steps == max_steps {
                continue;
            }

            for input in choices.expansion_choices(&history) {
                if let Ok(later) = self.expand(&history, &input)
                    && !histories.contains(&later)
                {
                    histories.push(later.clone());
                    pending.push_back((later, steps.saturating_add(1)));
                }
            }
        }

        histories
    }

    /// Reports whether `later` is produced from `earlier` within `max_steps` expansions.
    fn expansion_reaches<Choices>(&self, earlier: &H, later: &H, choices: &Choices, max_steps: usize) -> bool
    where
        Choices: ExpansionChoices<H, This::Input>,
    {
        self.expansion_future_cone(earlier, choices, max_steps).contains(later)
    }
}

/// Composes constructive expansion with collapse over a bounded future cone.
#[ext(name = ExpansionCollapseExt)]
pub impl<This, H, Q, P> This
where
    This: HistoryExpansion<H> + HistoryCollapse<H, Q, P>,
    H: Clone + Eq,
    P: Clone + Eq,
{
    /// Returns the observation determined at `history` over its bounded admissible future cone.
    ///
    /// Follows paper Definition 2.6: some future produces an observation, and every future that
    /// produces one produces the same. A future without an observation does not prevent collapse.
    /// This operation is deliberately bounded and must not be presented as unbounded semantic
    /// closure.
    fn bounded_semantic_collapse<Choices>(
        &self,
        history: &H,
        query: &Q,
        choices: &Choices,
        max_steps: usize,
    ) -> Option<P>
    where
        Choices: ExpansionChoices<H, This::Input>,
    {
        let mut observations = self.bounded_observations(history, query, choices, max_steps).into_iter();
        let first = observations.next()?;
        observations.all(|observation| observation == first).then_some(first)
    }

    /// Returns the defined observations of every history in the bounded admissible future cone.
    fn bounded_observations<Choices>(&self, history: &H, query: &Q, choices: &Choices, max_steps: usize) -> Vec<P>
    where
        Choices: ExpansionChoices<H, This::Input>,
    {
        self.expansion_future_cone(history, choices, max_steps)
            .into_iter()
            .filter_map(|future| self.collapse_history(&future, query))
            .collect()
    }
}

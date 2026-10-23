use alux_ext::ext;
use std::collections::{HashSet, VecDeque};
use std::hash::Hash;
use std::iter::successors;

/// Derives cycle-safe predecessor chains from anchor values.
#[ext(name = PredecessorChainExt)]
pub impl<This, C> This
where
    This: Fn(&C) -> Option<C>,
    C: Clone + Eq + Hash,
{
    /// Returns the predecessor closure of `anchor` in oldest-first order.
    fn predecessor_chain(&self, anchor: C) -> Vec<C> {
        let mut seen = HashSet::new();

        successors(Some(anchor), self)
            .take_while(|candidate| seen.insert(candidate.clone()))
            .fold(VecDeque::new(), |mut chain, candidate| {
                chain.push_front(candidate);
                chain
            })
            .into_iter()
            .collect()
    }
}

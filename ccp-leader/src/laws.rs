use crate::{CandidateAuthor, LeaderSelection, LeaderSelectionPolicy};
use alux_ext::ext;
use anyhow::{Result, anyhow, ensure};
use ccp_core::CandidateSpace;
use ccp_validator::ValidatorSet;
use std::{collections::HashSet, fmt::Debug, hash::Hash};

/// Checks periodic leader selection against ordered committee membership.
#[ext(name = PeriodicLeaderLawsExt)]
pub impl<This, V> This
where
    This: LeaderSelectionPolicy<u64, V>,
    V: Clone + Eq,
{
    /// Checks two committee rotations, intervening rounds, and the initial unscheduled prefix.
    fn check_periodic_leaders(&self, validators: &[V], first: u64, period: u64) -> Result<()> {
        ensure!(period > 0, "periodic leader law requires a positive cadence");
        let waves = validators.len().saturating_mul(2).max(1) as u64;
        let last = waves
            .checked_mul(period)
            .and_then(|span| first.checked_add(span))
            .ok_or_else(|| anyhow!("leader law rounds exceed the slot domain"))?;
        (0..last).try_for_each(|round| {
            let expected = match round.checked_sub(first) {
                Some(offset) if offset.is_multiple_of(period) => usize::try_from(offset / period)
                    .ok()
                    .and_then(|wave| validators.get(wave % validators.len().max(1)))
                    .map_or(LeaderSelection::Unresolved, |validator| {
                        LeaderSelection::Selected(vec![validator.clone()])
                    }),
                _ => LeaderSelection::NotScheduled,
            };
            ensure!(
                self.select_leaders(&round, validators) == expected,
                "leader selection at round {round} disagrees with periodic committee rotation"
            );
            Ok(())
        })
    }
}

/// Checks authenticated candidate authorship against a validator set.
#[ext(name = CandidateAuthorLawsExt)]
pub impl<This, V> This
where
    This: CandidateAuthor<Validator = V> + CandidateSpace + ValidatorSet,
{
    /// Checks that every candidate has an author in the validator set.
    fn check_candidate_authors(&self) -> Result<()>
    where
        V: Clone + Eq + Hash + Debug,
        This::Candidate: Clone + Debug,
    {
        let validators = self.validators().into_iter().collect::<HashSet<_>>();
        self.slots().into_iter().flat_map(|slot| self.candidates(&slot)).try_for_each(|candidate| {
            let author = self
                .candidate_author(&candidate)
                .ok_or_else(|| anyhow!("candidate has no authenticated author: {candidate:?}"))?;
            ensure!(validators.contains(&author), "candidate author is outside the validator set: {author:?}");
            Ok(())
        })
    }
}

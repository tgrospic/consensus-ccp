use crate::{CommitteeOrder, QuorumThreshold, QuorumThresholdExt, ValidatorSet};
use alux_ext::ext;
use anyhow::{Result, ensure};
use std::{collections::HashSet, fmt::Debug, hash::Hash};

/// Checks that leader-selection order denotes exactly the validator set.
#[ext(name = CommitteeLawsExt)]
pub impl<This, V> This
where
    This: CommitteeOrder<Validator = V> + ValidatorSet,
{
    /// Checks that committee order contains every validator exactly once.
    fn check_committee_order(&self) -> Result<()>
    where
        V: Clone + Eq + Hash + Debug,
    {
        self.check_validator_set()?;
        let validators = self.validators().into_iter().collect::<HashSet<_>>();
        let order = self.committee_order();
        let ordered = order.iter().cloned().collect::<HashSet<_>>();
        ensure!(order.len() == ordered.len(), "committee order contains duplicate validators: {order:?}");
        ensure!(validators == ordered, "committee order does not denote the validator set: {order:?}");
        Ok(())
    }
}

/// Checks validator-set obligations shared by Byzantine protocols.
#[ext(name = ValidatorSetLawsExt)]
pub impl<This, V> This
where
    This: ValidatorSet<Validator = V>,
{
    /// Checks that an evidence view contains distinct validator identities.
    fn check_validator_set(&self) -> Result<()>
    where
        V: Eq + Hash,
    {
        let validators = self.validators();
        ensure!(
            validators.iter().collect::<HashSet<_>>().len() == validators.len(),
            "validator set contains duplicate identities"
        );
        Ok(())
    }
}

/// Checks shared quorum-threshold obligations.
#[ext(name = QuorumLawsExt)]
pub impl<This> This
where
    This: QuorumThreshold,
{
    /// Checks that a quorum threshold is defined and monotone for `total` participants.
    fn check_quorum_threshold(&self, total: usize) -> Result<()> {
        ensure!(
            self.threshold_numerator() != 0 && self.threshold_denominator() != 0,
            "quorum threshold is zero or undefined"
        );
        ensure!(
            self.threshold_numerator() <= self.threshold_denominator(),
            "quorum threshold exceeds the whole validator set"
        );
        ensure!(!self.meets_quorum(0, total), "empty support forms a quorum");
        if total == 0 {
            ensure!(!self.meets_quorum(1, total), "empty committee forms a quorum");
        } else {
            let required = self.quorum_size(total);
            ensure!(required <= total, "quorum size exceeds the validator set");
            ensure!(!self.meets_quorum(required.saturating_sub(1), total), "quorum accepts insufficient support");
            ensure!(self.meets_quorum(required, total), "quorum rejects its required support");
        }
        let numerator = u128::from(self.threshold_numerator());
        let denominator = u128::from(self.threshold_denominator());
        (0..=total).try_for_each(|supported| {
            let strict = total != 0 && supported as u128 * denominator > total as u128 * numerator;
            ensure!(
                self.meets_quorum(supported, total) == strict,
                "quorum of {supported}/{total} disagrees with the strict threshold fraction"
            );
            Ok(())
        })
    }
}

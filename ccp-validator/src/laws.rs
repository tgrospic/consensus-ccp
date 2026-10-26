use crate::{CommitteeOrder, QuorumThreshold, QuorumThresholdExt, ValidatorSet, ValidatorWeight, WeightedCommitteeExt};
use alux_ext::ext;
use anyhow::{Result, ensure};
use std::{collections::HashSet, fmt::Debug, hash::Hash};

/// Checks validator-set obligations shared by Byzantine protocols.
#[ext(name = ValidatorSetLawsExt)]
pub impl<This, V> This
where
    This: ValidatorSet<Validator = V>,
{
    /// Checks that a committee contains distinct validator identities.
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

/// Checks shared quorum-threshold obligations.
#[ext(name = QuorumLawsExt)]
pub impl<This> This
where
    This: QuorumThreshold,
{
    /// Checks that a quorum threshold is defined, strict below the whole weight, and monotone for
    /// `total` weight, and that the whole weight is always a quorum.
    fn check_quorum_threshold(&self, total: u128) -> Result<()> {
        ensure!(
            self.threshold_numerator() != 0 && self.threshold_denominator() != 0,
            "quorum threshold is zero or undefined"
        );
        ensure!(
            self.threshold_numerator() <= self.threshold_denominator(),
            "quorum threshold exceeds the whole committee"
        );
        ensure!(!self.meets_quorum(0, total), "empty support forms a quorum");
        if total == 0 {
            ensure!(!self.meets_quorum(1, total), "empty committee forms a quorum");
        } else {
            let required = self.quorum_size(total);
            ensure!(required <= total, "quorum size exceeds the committee weight");
            ensure!(!self.meets_quorum(required - 1, total), "quorum accepts insufficient support");
            ensure!(self.meets_quorum(required, total), "quorum rejects its required support");
            ensure!(self.fault_bound(total) == total - required, "fault bound is not the weight outside a quorum");
        }
        let numerator = u128::from(self.threshold_numerator());
        let denominator = u128::from(self.threshold_denominator());
        (0..=total).try_for_each(|supported| {
            let strict = total != 0 && (supported == total || supported * denominator > total * numerator);
            ensure!(
                self.meets_quorum(supported, total) == strict,
                "quorum of {supported}/{total} disagrees with the threshold fraction"
            );
            Ok(())
        })
    }
}

/// Checks that weighted support counts each committee member once by its weight.
#[ext(name = WeightedSupportLawsExt)]
pub impl<This, V, W> This
where
    This: ValidatorSet<Validator = V> + ValidatorWeight<Weight = W>,
    W: Into<u128>,
{
    /// Checks weighted support of `supporters` and `others` under `threshold`.
    ///
    /// Empty support weighs nothing and never forms a quorum, repeated supporters count once,
    /// support never exceeds the committee, adding supporters never loses a quorum, two quorums
    /// share weight when the threshold is at least one half, and unit weights agree with counting.
    fn check_weighted_support<Threshold>(&self, threshold: &Threshold, supporters: &[V], others: &[V]) -> Result<()>
    where
        Threshold: QuorumThreshold,
        V: Clone + Eq + Hash + Debug,
    {
        let total = self.total_weight();
        let weight = self.support_weight(supporters.iter().cloned());
        ensure!(self.support_weight([]) == 0, "empty support has weight");
        ensure!(!self.weighted_quorum(threshold, []), "empty support forms a quorum");
        ensure!(
            self.support_weight(supporters.iter().chain(supporters).cloned()) == weight,
            "repeated supporters count more than once: {supporters:?}"
        );
        ensure!(weight <= total, "support exceeds the committee weight: {supporters:?}");

        let union = supporters.iter().chain(others).cloned().collect::<Vec<_>>();
        ensure!(weight <= self.support_weight(union.iter().cloned()), "adding supporters lost weight");
        ensure!(
            !self.weighted_quorum(threshold, supporters.iter().cloned())
                || self.weighted_quorum(threshold, union.iter().cloned()),
            "adding supporters lost a quorum: {supporters:?} + {others:?}"
        );

        if threshold.quorums_intersect()
            && self.weighted_quorum(threshold, supporters.iter().cloned())
            && self.weighted_quorum(threshold, others.iter().cloned())
        {
            let shared = supporters.iter().filter(|validator| others.contains(validator)).cloned();
            ensure!(self.support_weight(shared) > 0, "quorums {supporters:?} and {others:?} share no weight");
        }

        let validators = self.validators();
        if validators.iter().all(|validator| self.weight(validator).into() == 1) {
            let counted = supporters.iter().filter(|validator| validators.contains(validator)).collect::<HashSet<_>>();
            ensure!(total == validators.len() as u128, "unit committee weight disagrees with its size");
            ensure!(weight == counted.len() as u128, "unit support weight disagrees with counting");
        }
        Ok(())
    }
}

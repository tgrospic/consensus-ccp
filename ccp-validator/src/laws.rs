use crate::{
    ChurnLimit, CommitteeOrder, CommitteeRotationExt, CommitteeSchedule, EpochBindingExt, HandoverThresholdExt,
    QuorumThreshold, QuorumThresholdExt, Reweighting, SlotEpoch, ValidatorSet, ValidatorWeight, WeightedCommitteeExt,
};
use alux_ext::ext;
use anyhow::{Result, ensure};
use std::{
    collections::HashSet,
    fmt::Debug,
    hash::Hash,
    ops::{Add, Sub},
};

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
            ensure!(
                !self.quorums_outweigh_faults() || 2 * required > total + self.fault_bound(total),
                "two quorums of {required}/{total} can overlap within the fault bound"
            );
            ensure!(
                !self.quorums_outweigh_faults() || self.handover_fault_bound(total, 0) == self.fault_bound(total),
                "a handover without turnover changes the fault bound of {total}"
            );
            (0..=total).try_for_each(|turnover| {
                let bound = self.handover_fault_bound(total, turnover);
                ensure!(bound <= self.fault_bound(total), "turnover {turnover} raises the fault bound of {total}");
                ensure!(
                    self.handover_fault_bound(total, turnover + 1) <= bound,
                    "more turnover than {turnover} raises the handover fault bound of {total}"
                );
                Ok(())
            })?;
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
    /// share weight when the threshold is at least one half and share more than the fault bound
    /// from two thirds upward, and unit weights agree with counting.
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
            let shared = self.support_weight(supporters.iter().filter(|validator| others.contains(validator)).cloned());
            ensure!(shared > 0, "quorums {supporters:?} and {others:?} share no weight");
            ensure!(
                !threshold.quorums_outweigh_faults() || shared > threshold.fault_bound(total),
                "quorums {supporters:?} and {others:?} share no more than the fault bound"
            );
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

/// Checks accountable safety of conflicting commits backed by weighted quorums.
#[ext(name = AccountableSafetyLawsExt)]
pub impl<This, V, W> This
where
    This: ValidatorSet<Validator = V> + ValidatorWeight<Weight = W>,
    W: Into<u128>,
{
    /// Checks two commits, each a slot, a candidate, and the validators backing it.
    ///
    /// When both commits name the same slot and different candidates and each is backed by a
    /// quorum, the validators backing both are accountable for the conflict and weigh more than the
    /// fault bound. So a faulty coalition within the bound cannot finalize two conflicting
    /// candidates in one slot.
    fn check_commit_safety<Threshold, S, C>(
        &self,
        threshold: &Threshold,
        first: (&S, &C, &[V]),
        second: (&S, &C, &[V]),
    ) -> Result<()>
    where
        Threshold: QuorumThreshold,
        S: Eq + Debug,
        C: Eq + Debug,
        V: Clone + Eq + Hash + Debug,
    {
        let ((slot, candidate, backers), (other_slot, other_candidate, other_backers)) = (first, second);
        if !threshold.quorums_outweigh_faults()
            || slot != other_slot
            || candidate == other_candidate
            || !self.weighted_quorum(threshold, backers.iter().cloned())
            || !self.weighted_quorum(threshold, other_backers.iter().cloned())
        {
            return Ok(());
        }
        let accountable =
            backers.iter().filter(|validator| other_backers.contains(validator)).cloned().collect::<Vec<_>>();
        ensure!(
            self.support_weight(accountable.iter().cloned()) > threshold.fault_bound(self.total_weight()),
            "conflicting commits {candidate:?} and {other_candidate:?} at {slot:?} are accountable to no more \
             than the fault bound: {accountable:?}"
        );
        Ok(())
    }
}

/// Checks what validator rotation means: each epoch has a lawful committee, rotation is exactly the
/// difference between two committees, turnover is a distance, and quorums across a handover overlap by
/// at least what the threshold guarantees for its turnover.
#[ext(name = RotationLawsExt)]
pub impl<This, V, W, E> This
where
    This: CommitteeSchedule<Validator = V, Weight = W, Epoch = E>,
    V: Clone + Eq + Hash + Debug,
    W: Copy + Ord + Default + Debug + Add<Output = W> + Sub<Output = W> + Into<u128>,
    E: Clone + Debug,
{
    /// Checks that the committee of `epoch` lists each validator once, in an order that denotes it.
    fn check_epoch_committee(&self, epoch: &E) -> Result<()> {
        self.epoch_committee(epoch).check_committee_order()
    }

    /// Checks that an epoch does not rotate into itself.
    fn check_rotation_identity(&self, epoch: &E) -> Result<()> {
        let rotation = self.rotation(epoch, epoch);
        let still = rotation.entering.is_empty() && rotation.leaving.is_empty() && rotation.reweighted.is_empty();
        ensure!(still, "{epoch:?} rotates into itself by {rotation:?}");
        ensure!(self.turnover(epoch, epoch) == W::default(), "{epoch:?} turns over into itself");
        Ok(())
    }

    /// Checks that applying the rotation from one epoch to another to the first committee gives
    /// exactly the second, and that the rotation is empty exactly when nothing turns over.
    fn check_rotation_reconstructs(&self, from: &E, to: &E) -> Result<()> {
        let rotation = self.rotation(from, to);
        let mut committee: Vec<_> = self
            .committee_at(from)
            .into_iter()
            .filter(|(validator, _)| !rotation.leaving.iter().any(|(left, _)| left == validator))
            .map(|(validator, weight)| {
                let moved = rotation.reweighted.iter().find(|change| change.validator == validator);
                (validator, moved.map_or(weight, |change| change.to))
            })
            .collect();
        committee.extend(rotation.entering.iter().cloned());
        ensure!(same_members(&committee, &self.committee_at(to)), "rotating {from:?} does not give {to:?}");
        let still = rotation.entering.is_empty() && rotation.leaving.is_empty() && rotation.reweighted.is_empty();
        ensure!(
            !still || self.turnover(from, to) == W::default(),
            "{from:?} and {to:?} have the same committee but turn over"
        );
        Ok(())
    }

    /// Checks that rotating back swaps who enters and who leaves, and reverses every reweighting.
    fn check_rotation_inverse(&self, from: &E, to: &E) -> Result<()> {
        let forward = self.rotation(from, to);
        let backward = self.rotation(to, from);
        let reversed: Vec<_> = forward
            .reweighted
            .iter()
            .map(|change| Reweighting { validator: change.validator.clone(), from: change.to, to: change.from })
            .collect();
        ensure!(same_members(&backward.entering, &forward.leaving), "rotating back does not readmit who left");
        ensure!(same_members(&backward.leaving, &forward.entering), "rotating back does not remove who entered");
        ensure!(same_members(&backward.reweighted, &reversed), "rotating back does not reverse reweightings");
        Ok(())
    }

    /// Checks that turnover is a distance between epochs: zero to itself, symmetric, never shorter
    /// through an epoch in between, at least the change in total weight, and at most both totals.
    fn check_turnover_distance(&self, first: &E, second: &E, third: &E) -> Result<()> {
        let distance = |from, to| self.turnover(from, to);
        ensure!(distance(first, first) == W::default(), "turnover from {first:?} to itself is not zero");
        ensure!(distance(first, second) == distance(second, first), "turnover is not symmetric");
        let direct = distance(first, third);
        let through = distance(first, second) + distance(second, third);
        ensure!(direct <= through, "turnover {direct:?} exceeds {through:?} through {second:?}");
        let (from, to) = (self.epoch_total(first), self.epoch_total(second));
        let turnover = distance(first, second);
        ensure!(from.max(to) - from.min(to) <= turnover, "turnover {turnover:?} is below the change in total weight");
        ensure!(turnover <= from + to, "turnover {turnover:?} exceeds both committees");
        Ok(())
    }

    /// Checks the handover from `from` to `to` with `outgoing` and `incoming` as quorums.
    ///
    /// When each set is a quorum of its own committee, the validators in both weigh at least
    /// `handover_overlap` in `from`, so they outweigh any faulty weight within `handover_fault_bound`,
    /// and they weigh something whenever the quorums intersect.
    fn check_handover_overlap<Threshold>(
        &self,
        threshold: &Threshold,
        from: &E,
        to: &E,
        outgoing: &[V],
        incoming: &[V],
    ) -> Result<()>
    where
        Threshold: QuorumThreshold,
    {
        let (before, after) = (self.epoch_committee(from), self.epoch_committee(to));
        if !before.weighted_quorum(threshold, outgoing.iter().cloned())
            || !after.weighted_quorum(threshold, incoming.iter().cloned())
        {
            return Ok(());
        }
        let shared = outgoing.iter().filter(|validator| incoming.contains(validator)).cloned().collect::<Vec<_>>();
        let shared = before.support_weight(shared.iter().cloned());
        let (total, turnover) = (self.epoch_total(from).into(), self.turnover(from, to).into());
        let overlap = threshold.handover_overlap(total, turnover);
        ensure!(
            shared >= overlap,
            "quorums {outgoing:?} of {from:?} and {incoming:?} of {to:?} share {shared}, below the overlap {overlap}"
        );
        ensure!(
            !self.handover_quorums_intersect(threshold, from, to) || shared > 0,
            "quorums {outgoing:?} of {from:?} and {incoming:?} of {to:?} share no weight"
        );
        let faults = threshold.handover_fault_bound(total, turnover);
        ensure!(overlap == 0 || shared > faults, "the overlap {shared} does not outweigh {faults} faulty weight");
        Ok(())
    }

    /// Checks that rotating from `from` to `to` stays within `limit`, that a limit the threshold
    /// tolerates makes quorums across the handover intersect, and that staying within the limit keeps
    /// at least the fault bound the limit leaves.
    fn check_churn<Limit, Threshold>(&self, limit: &Limit, threshold: &Threshold, from: &E, to: &E) -> Result<()>
    where
        Limit: ChurnLimit,
        Threshold: QuorumThreshold,
    {
        let total = self.epoch_total(from).into();
        let turnover = self.turnover(from, to).into();
        let allowed = limit.churn_limit(total);
        ensure!(turnover <= allowed, "{from:?} to {to:?} turns over {turnover}, beyond the limit {allowed}");
        ensure!(
            !threshold.handover_intersects(total, allowed) || self.handover_quorums_intersect(threshold, from, to),
            "quorums across a handover within the churn limit do not intersect from {from:?} to {to:?}"
        );
        ensure!(
            threshold.handover_fault_bound(total, turnover) >= threshold.handover_fault_bound(total, allowed),
            "{from:?} to {to:?} tolerates fewer faults than its churn limit leaves"
        );
        Ok(())
    }
}

/// Checks that a churn limit keeps quorums intersecting across a handover.
#[ext(name = ChurnLimitLawsExt)]
pub impl<This> This
where
    This: ChurnLimit,
{
    /// Checks that, for a committee of `total` weight, quorums across a handover within the limit
    /// intersect under `threshold`, and that the limit grows with the committee and never exceeds it.
    fn check_churn_limit<Threshold>(&self, threshold: &Threshold, total: u128) -> Result<()>
    where
        Threshold: QuorumThreshold,
    {
        let allowed = self.churn_limit(total);
        ensure!(allowed <= total, "churn limit {allowed} exceeds the committee weight {total}");
        ensure!(
            total == 0 || threshold.handover_intersects(total, allowed),
            "churn limit {allowed} of {total} is beyond what the threshold tolerates"
        );
        ensure!(
            total == 0 || self.churn_limit(total - 1) <= allowed,
            "churn limit shrinks as the committee grows to {total}"
        );
        Ok(())
    }
}

/// Checks that evidence is weighed by the committee of its own epoch, which keeps safety a property
/// of each epoch however much rotates between epochs.
#[ext(name = EpochBindingLawsExt)]
pub impl<This, V, W, E, S> This
where
    This: CommitteeSchedule<Validator = V, Weight = W, Epoch = E> + SlotEpoch<Slot = S>,
    V: Clone + Eq + Hash + Debug,
    W: Copy + Ord + Default + Debug + Add<Output = W> + Sub<Output = W> + Into<u128>,
    E: Clone + Ord + Debug,
{
    /// Checks that a later slot never falls in an earlier epoch.
    fn check_slot_epochs(&self, earlier: &S, later: &S) -> Result<()>
    where
        S: Ord + Debug,
    {
        let (first, second) = (self.epoch_of(earlier), self.epoch_of(later));
        ensure!(earlier > later || first <= second, "{later:?} in {second:?} falls before {earlier:?} in {first:?}");
        Ok(())
    }

    /// Checks that `view`, the committee a protocol weighs evidence of `slot` with, is the committee
    /// of the slot's epoch: the same validators with the same weights.
    fn check_committee_binding<View>(&self, view: &View, slot: &S) -> Result<()>
    where
        View: ValidatorSet<Validator = V> + ValidatorWeight<Weight = W>,
        S: Debug,
    {
        let committee = self.slot_committee(slot);
        let epoch = self.epoch_of(slot);
        ensure!(
            same_members(&view.validators(), &committee.validators()),
            "evidence of {slot:?} is weighed by {:?}, not the committee of {epoch:?}",
            view.validators()
        );
        view.validators().iter().try_for_each(|validator| {
            let (weighed, expected) = (view.weight(validator), committee.weight(validator));
            ensure!(
                weighed == expected,
                "{validator:?} weighs {weighed:?} for {slot:?}, not {expected:?} of {epoch:?}"
            );
            Ok(())
        })
    }

    /// Checks accountable safety of two commits weighed by the committee of their slot's epoch: when
    /// both are in one slot, name different candidates, and are quorums of that committee, their
    /// shared backers weigh more than its fault bound.
    fn check_epoch_commit_safety<Threshold, C>(
        &self,
        threshold: &Threshold,
        first: (&S, &C, &[V]),
        second: (&S, &C, &[V]),
    ) -> Result<()>
    where
        Threshold: QuorumThreshold,
        S: Eq + Debug,
        C: Eq + Debug,
    {
        self.slot_committee(first.0).check_commit_safety(threshold, first, second)
    }
}

/// Says whether two lists hold the same distinct items, in any order.
fn same_members<T: PartialEq>(left: &[T], right: &[T]) -> bool {
    left.len() == right.len() && left.iter().all(|item| right.contains(item))
}

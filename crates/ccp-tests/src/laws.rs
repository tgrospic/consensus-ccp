//! Runs the shared committee, quorum, leader, and predecessor laws against concrete reference policies.
use anyhow::{Result, ensure};
use ccp_core::{
    Closed, ExpansionChoices, ExpansionCollapseExt, ExpansionCollapseLawsExt, HistoryCollapse, HistoryExpansion,
    PredecessorChainLawsExt,
};
use ccp_leader::{PeriodicLeaderLawsExt, PeriodicRoundRobin};
use ccp_validator::{
    AccountableSafetyLawsExt, ChurnLimit, ChurnLimitLawsExt, CommitteeLawsExt, CommitteeOrder, CommitteeRotationExt,
    CommitteeSchedule, EpochSorts, HandoverThresholdExt, QuorumLawsExt, QuorumThreshold, QuorumThresholdExt,
    RotationLawsExt, TwoThirdsQuorum, ValidatorSet, ValidatorSorts, ValidatorWeight, WeightedSupportLawsExt,
};

// A committee with unequal voting weights, listed in committee order.
struct WeightedCommittee(Vec<(u8, u64)>);

impl ValidatorSorts for WeightedCommittee {
    type Validator = u8;
    type Weight = u64;
}

impl ValidatorSet for WeightedCommittee {
    fn validators(&self) -> Vec<u8> {
        self.0.iter().map(|(validator, _)| *validator).collect()
    }
}

impl CommitteeOrder for WeightedCommittee {
    fn committee_order(&self) -> Vec<u8> {
        self.validators()
    }
}

impl ValidatorWeight for WeightedCommittee {
    fn weight(&self, validator: &u8) -> u64 {
        self.0.iter().find(|(member, _)| member == validator).map_or(0, |(_, weight)| *weight)
    }
}

struct Fraction(u64, u64);

impl QuorumThreshold for Fraction {
    fn threshold_numerator(&self) -> u64 {
        self.0
    }

    fn threshold_denominator(&self) -> u64 {
        self.1
    }
}

#[test]
fn quorum_thresholds_satisfy_threshold_laws() -> Result<()> {
    (0..=12).try_for_each(|total| TwoThirdsQuorum.check_quorum_threshold(total))?;
    [(1, 2), (1, 3), (3, 4), (99, 100), (1, 1)].into_iter().try_for_each(|(numerator, denominator)| {
        (0..=12).try_for_each(|total| Fraction(numerator, denominator).check_quorum_threshold(total))
    })
}

#[test]
fn weighted_committees_satisfy_support_laws() -> Result<()> {
    let committees = [
        WeightedCommittee(vec![(0, 1), (1, 1), (2, 1), (3, 1)]),
        WeightedCommittee(vec![(0, 5), (1, 3), (2, 2), (3, 1)]),
        WeightedCommittee(vec![(0, 10), (1, 1), (2, 1)]),
        WeightedCommittee(vec![(0, 0), (1, 2), (2, 2)]),
    ];
    // Validator 9 lies outside every committee, so support must ignore it.
    let candidates = [vec![], vec![0], vec![1, 2], vec![0, 0, 3], vec![1, 2, 3], vec![0, 1, 2, 3], vec![9, 1]];
    let thresholds = [Fraction(2, 3), Fraction(3, 4), Fraction(1, 1), Fraction(1, 2), Fraction(1, 3)];
    committees.iter().try_for_each(|committee| {
        committee.check_committee_order()?;
        thresholds.iter().try_for_each(|threshold| {
            candidates.iter().try_for_each(|supporters| {
                candidates.iter().try_for_each(|others| {
                    committee.check_weighted_support(threshold, supporters, others)?;
                    committee.check_commit_safety(threshold, (&0, &'a', supporters), (&0, &'b', others))
                })
            })
        })
    })
}

#[test]
fn periodic_round_robin_satisfies_rotation_laws() -> Result<()> {
    [(0, 1), (0, 3), (2, 3), (5, 2)].into_iter().try_for_each(|(first, period)| {
        (0..=4).try_for_each(|size| {
            let validators = (0..size).collect::<Vec<u8>>();
            PeriodicRoundRobin::new(first, period).check_periodic_leaders(&validators, first, period)
        })
    })
}

#[test]
fn predecessor_chains_satisfy_closure_laws() -> Result<()> {
    let acyclic = |candidate: &u8| candidate.checked_sub(1);
    let cyclic = |candidate: &u8| match candidate {
        3 | 1 => Some(2),
        2 => Some(1),
        _ => None,
    };
    (0..=4).try_for_each(|anchor| acyclic.check_predecessor_chain(anchor))?;
    (0..=4).try_for_each(|anchor| cyclic.check_predecessor_chain(anchor))
}

// A history that counts its expansions; each collapse reads the count.
struct Counter;

impl HistoryExpansion<u8> for Counter {
    type Input = ();
    type Error = ();

    fn expand(&self, history: &u8, _input: &()) -> Result<u8, ()> {
        Ok(history + 1)
    }
}

impl ExpansionChoices<u8, ()> for Counter {
    fn expansion_choices(&self, _history: &u8) -> Vec<()> {
        vec![()]
    }
}

// Commits the count itself, so later histories collapse differently.
struct UnsafeCollapse;

impl HistoryCollapse<u8, (), Closed<u8>> for UnsafeCollapse {
    fn collapse_history(&self, history: &u8, _query: &()) -> Option<Closed<u8>> {
        Some(Closed::Commit(*history))
    }
}

// Commits candidate 7 once one expansion is seen, and leaves the initial history undecided.
struct DeferredCollapse;

impl HistoryCollapse<u8, (), Closed<u8>> for DeferredCollapse {
    fn collapse_history(&self, history: &u8, _query: &()) -> Option<Closed<u8>> {
        (*history > 0).then_some(Closed::Commit(7))
    }
}

struct WithCollapse<Collapse>(Collapse);

impl<Collapse> HistoryExpansion<u8> for WithCollapse<Collapse> {
    type Input = ();
    type Error = ();

    fn expand(&self, history: &u8, input: &()) -> Result<u8, ()> {
        Counter.expand(history, input)
    }
}

impl<Collapse> HistoryCollapse<u8, (), Closed<u8>> for WithCollapse<Collapse>
where
    Collapse: HistoryCollapse<u8, (), Closed<u8>>,
{
    fn collapse_history(&self, history: &u8, query: &()) -> Option<Closed<u8>> {
        self.0.collapse_history(history, query)
    }
}

#[test]
fn collapse_agreement_rejects_disagreeing_futures() -> Result<()> {
    WithCollapse(DeferredCollapse).check_collapse_agreement(&0, &[()], &Counter, 3)?;
    ensure!(WithCollapse(UnsafeCollapse).check_collapse_agreement(&0, &[()], &Counter, 3).is_err());
    Ok(())
}

#[test]
fn semantic_collapse_ignores_undefined_futures() {
    assert_eq!(WithCollapse(DeferredCollapse).bounded_semantic_collapse(&0, &(), &Counter, 2), Some(Closed::Commit(7)));
    assert_eq!(WithCollapse(DeferredCollapse).bounded_semantic_collapse(&0, &(), &Counter, 0), None);
    assert_eq!(WithCollapse(UnsafeCollapse).bounded_semantic_collapse(&0, &(), &Counter, 2), None);
}

#[test]
fn commit_safety_holds_only_from_two_thirds() -> Result<()> {
    let committee = WeightedCommittee(vec![(0, 1), (1, 1), (2, 1), (3, 1)]);
    let (left, right) = ([0, 1, 2], [1, 2, 3]);
    committee.check_commit_safety(&Fraction(2, 3), (&0, &'a', &left[..]), (&0, &'b', &right[..]))?;
    ensure!(!Fraction(1, 2).quorums_outweigh_faults() && Fraction(2, 3).quorums_outweigh_faults());
    Ok(())
}

// A committee per epoch, each listed in committee order with unequal weights.
struct Epochs(Vec<Vec<(u8, u64)>>);

impl ValidatorSorts for Epochs {
    type Validator = u8;
    type Weight = u64;
}

impl EpochSorts for Epochs {
    type Epoch = usize;
}

impl CommitteeSchedule for Epochs {
    fn committee_at(&self, epoch: &usize) -> Vec<(u8, u64)> {
        self.0.get(*epoch).cloned().unwrap_or_default()
    }
}

// Lets at most one fifth of the weight, rounded down and kept strictly below, rotate per epoch.
struct FifthOfWeight;

impl ChurnLimit for FifthOfWeight {
    fn churn_limit(&self, total: u128) -> u128 {
        total.saturating_sub(1) / 5
    }
}

#[test]
fn committee_schedules_satisfy_rotation_laws() -> Result<()> {
    let schedule = Epochs(vec![
        vec![(0, 3), (1, 3), (2, 2), (3, 1), (4, 1)],
        vec![(0, 3), (1, 3), (2, 2), (3, 1), (4, 2)],
        vec![(0, 3), (1, 2), (2, 2), (3, 1), (4, 2)],
        vec![(5, 4), (6, 3)],
        vec![],
    ]);
    let members = [0, 1, 2, 3, 4, 5, 6];
    let quorums = (0..1u32 << members.len())
        .map(|mask| members.iter().enumerate().filter(|(index, _)| mask & (1 << index) != 0).map(|(_, v)| *v).collect())
        .collect::<Vec<Vec<u8>>>();
    let epochs = 0..schedule.0.len();
    for first in epochs.clone() {
        schedule.check_epoch_committee(&first)?;
        schedule.check_rotation_identity(&first)?;
        for second in epochs.clone() {
            schedule.check_rotation_reconstructs(&first, &second)?;
            schedule.check_rotation_inverse(&first, &second)?;
            epochs.clone().try_for_each(|third| schedule.check_turnover_distance(&first, &second, &third))?;
            for threshold in [Fraction(2, 3), Fraction(3, 4), Fraction(1, 1)] {
                for outgoing in &quorums {
                    for incoming in &quorums {
                        schedule.check_handover_overlap(&threshold, &first, &second, outgoing, incoming)?;
                    }
                }
            }
        }
    }
    schedule.check_churn(&FifthOfWeight, &TwoThirdsQuorum, &0, &1)?;
    schedule.check_churn(&FifthOfWeight, &TwoThirdsQuorum, &1, &2)?;
    ensure!(schedule.check_churn(&FifthOfWeight, &TwoThirdsQuorum, &2, &3).is_err(), "a full rotation fits the churn");
    ensure!(schedule.handover_quorums_intersect(&TwoThirdsQuorum, &0, &1), "rotating one unit of ten splits quorums");
    ensure!(!schedule.handover_quorums_intersect(&TwoThirdsQuorum, &2, &3), "replacing the committee keeps quorums");
    Ok(())
}

#[test]
fn turnover_spends_the_fault_bound() -> Result<()> {
    let bounds: Vec<u128> = (0..7).map(|turnover| TwoThirdsQuorum.handover_fault_bound(24, turnover)).collect();
    ensure!(bounds == [7, 7, 5, 4, 2, 0, 0], "the fault bound of 24 across turnover is {bounds:?}");
    ensure!(TwoThirdsQuorum.handover_overlap(24, 0) == 10, "two quorums of 17 in 24 share less than 10");
    ensure!(TwoThirdsQuorum.handover_intersects(24, 4), "quorums split at a sixth of the weight");
    ensure!(TwoThirdsQuorum.handover_fault_bound(24, 4) < TwoThirdsQuorum.fault_bound(24), "turnover was free");
    Ok(())
}

#[test]
fn churn_limits_keep_quorums_intersecting() -> Result<()> {
    (0..=60).try_for_each(|total| FifthOfWeight.check_churn_limit(&TwoThirdsQuorum, total))?;
    ensure!(
        (5..=60).any(|total| FifthOfWeight.check_churn_limit(&Fraction(3, 4), total).is_ok()),
        "a stricter threshold tolerates no churn"
    );
    ensure!(
        (6..=60).all(|total| FifthOfWeight.check_churn_limit(&Fraction(1, 2), total).is_err()),
        "one half tolerates churn"
    );
    Ok(())
}

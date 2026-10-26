//! Runs the shared committee, quorum, leader, and predecessor laws against concrete reference policies.
use anyhow::Result;
use ccp_core::PredecessorChainLawsExt;
use ccp_leader::{PeriodicLeaderLawsExt, PeriodicRoundRobin};
use ccp_validator::{
    CommitteeLawsExt, CommitteeOrder, QuorumLawsExt, QuorumThreshold, TwoThirdsQuorum, ValidatorSet, ValidatorSorts,
    ValidatorWeight, WeightedSupportLawsExt,
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
    let thresholds = [Fraction(2, 3), Fraction(1, 2), Fraction(1, 3)];
    committees.iter().try_for_each(|committee| {
        committee.check_committee_order()?;
        thresholds.iter().try_for_each(|threshold| {
            candidates.iter().try_for_each(|supporters| {
                candidates.iter().try_for_each(|others| committee.check_weighted_support(threshold, supporters, others))
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

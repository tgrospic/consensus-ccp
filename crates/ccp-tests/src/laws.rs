//! Runs the shared quorum, leader, and predecessor laws against concrete reference policies.
use anyhow::Result;
use ccp_core::PredecessorChainLawsExt;
use ccp_leader::{PeriodicLeaderLawsExt, PeriodicRoundRobin};
use ccp_quorum::{QuorumLawsExt, TwoThirdsQuorum};

#[test]
fn two_thirds_quorum_satisfies_threshold_laws() -> Result<()> {
    (0..=12).try_for_each(|total| TwoThirdsQuorum.check_quorum_threshold(total))
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

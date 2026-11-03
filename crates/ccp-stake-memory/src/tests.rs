use crate::{MemoryStake, Policy, Source};
use anyhow::{Result, ensure};
use ccp_stake::{
    StakeAccountabilityLawsExt, StakeChurnExt, StakeChurnLawsExt, StakeDelegationExt, StakeDelegationLawsExt,
    StakeEscrowExt, StakeEscrowLawsExt, StakeForfeitExt, StakeForfeitLawsExt, StakeQuorumExt, StakeQuorumLawsExt,
    StakeRewardsLawsExt, StakeSettlementLawsExt, StakeWeightExt, StakeWeightLawsExt, UnbondingAlg,
};
use ccp_validator::{ChurnLimitLawsExt, CommitteeRotationExt, RotationLawsExt, TwoThirdsQuorum};

const DELEGATORS: [char; 3] = ['x', 'y', 'z'];
const VALIDATORS: [char; 3] = ['a', 'b', 'c'];
const AMOUNTS: [u64; 4] = [0, 1, 40, 1_000];

fn bonded(policy: Policy) -> MemoryStake<char> {
    let mut stake = MemoryStake::new(policy);
    for (delegator, validator, amount) in [('x', 'a', 40), ('y', 'a', 20), ('y', 'b', 30), ('z', 'c', 10)] {
        stake.wallet_fund(&delegator, 100);
        stake.stake_bond(&delegator, &validator, amount).unwrap();
    }
    stake
}

fn delegated(policy: Policy) -> MemoryStake<char> {
    let mut stake = MemoryStake::new(Policy { source: Source::Delegation, ..policy });
    for (holder, validator, amount) in [('x', 'a', 40), ('y', 'a', 20), ('z', 'b', 30)] {
        stake.wallet_fund(&holder, amount);
        stake.stake_delegate(&holder, Some(validator));
    }
    stake
}

fn escrow_is_held(stake: &MemoryStake<char>) -> Result<()> {
    let pending: u64 =
        DELEGATORS.iter().flat_map(|delegator| stake.unbonding(delegator)).map(|unbonding| unbonding.amount).sum();
    let bonded: u64 = stake.escrow_weights().iter().map(|(_, weight)| weight).sum();
    ensure!(stake.vault() == bonded + pending, "the vault holds {} for {}", stake.vault(), bonded + pending);
    Ok(())
}

#[test]
fn snapshots_satisfy_the_weight_laws_for_both_sources() -> Result<()> {
    for lag in [1, 2] {
        for mut stake in [bonded(Policy { lag, ..Policy::default() }), delegated(Policy { lag, ..Policy::default() })] {
            for epoch in 0..4 {
                stake.check_snapshot()?;
                stake.check_weights(&epoch)?;
                stake.epoch_advance();
            }
        }
    }
    Ok(())
}

#[test]
fn exclusion_satisfies_the_weight_laws() -> Result<()> {
    for validator in VALIDATORS {
        for until in 0..3 {
            let mut stake = bonded(Policy::default());
            stake.check_exclusion(&validator, until)?;
            stake.epoch_advance();
            stake.check_snapshot()?;
        }
    }
    Ok(())
}

#[test]
fn quorum_satisfies_its_laws() -> Result<()> {
    let mut stake = bonded(Policy::default());
    stake.stake_snapshot_take();
    let subsets: Vec<Vec<char>> = (0..8)
        .map(|mask| VALIDATORS.iter().enumerate().filter(|(bit, _)| mask >> bit & 1 == 1).map(|(_, v)| *v).collect())
        .collect();
    for voters in &subsets {
        for others in &subsets {
            stake.check_epoch_quorum(&0, voters, others)?;
        }
    }
    Ok(())
}

#[test]
fn rewards_satisfy_their_laws() -> Result<()> {
    for performance in [(0, 0), (4, 10), (10, 10), (12, 10)] {
        let mut stake = bonded(Policy::default());
        stake.stake_snapshot_take();
        for validator in VALIDATORS {
            stake.blocks_observe(validator, 0, performance.0, performance.1);
        }
        stake.check_reward(&0)?;
        escrow_is_held(&stake)?;
    }
    Ok(())
}

#[test]
fn escrow_satisfies_its_laws() -> Result<()> {
    for delegator in DELEGATORS {
        for validator in VALIDATORS {
            for amount in AMOUNTS {
                let mut stake = bonded(Policy::default());
                stake.check_bond(&delegator, &validator, amount)?;
                stake.check_unbond(&delegator, &validator, amount)?;
                escrow_is_held(&stake)?;
                for _ in 0..3 {
                    stake.check_withdraw(&delegator)?;
                    stake.epoch_advance();
                }
                escrow_is_held(&stake)?;
            }
        }
    }
    Ok(())
}

#[test]
fn forfeiting_satisfies_its_laws() -> Result<()> {
    for validator in VALIDATORS {
        for basis_points in [0, 2_500, 10_000, 20_000] {
            let mut stake = bonded(Policy::default());
            stake.check_unbond(&'y', &'a', 10)?;
            stake.check_forfeit(&validator, &basis_points)?;
            escrow_is_held(&stake)?;
            stake.epoch_advance();
            stake.epoch_advance();
            stake.check_forfeit(&validator, &basis_points)?;
            escrow_is_held(&stake)?;
        }
    }
    Ok(())
}

#[test]
fn delegation_satisfies_its_laws() -> Result<()> {
    for holder in DELEGATORS {
        for validator in VALIDATORS.iter().copied().map(Some).chain([None]) {
            delegated(Policy::default()).check_delegate(&holder, validator)?;
        }
    }
    Ok(())
}

fn close(stake: &mut MemoryStake<char>) {
    stake.stake_snapshot_take();
    stake.epoch_advance();
}

/// Builds four epochs in which validators enter, leave, are excluded, and change weight.
fn rotating(lag: u64) -> MemoryStake<char> {
    let mut stake = bonded(Policy { lag, ..Policy::default() });
    stake.wallet_fund(&'z', 500);
    close(&mut stake);
    stake.stake_bond(&'z', &'d', 70).unwrap();
    stake.stake_unbond(&'x', 'a', 25).unwrap();
    close(&mut stake);
    stake.stake_exclude(&'b', 3);
    stake.stake_bond(&'x', &'c', 15).unwrap();
    close(&mut stake);
    stake.stake_unbond(&'z', 'c', 10).unwrap();
    close(&mut stake);
    close(&mut stake);
    stake
}

#[test]
fn every_epoch_after_genesis_is_governed_by_an_earlier_snapshot() -> Result<()> {
    for lag in [1, 2] {
        let stake = rotating(lag);
        (1..8).try_for_each(|epoch| stake.check_snapshot_precedes(&epoch))?;
    }
    Ok(())
}

#[test]
fn weights_are_settled_for_the_whole_epoch() -> Result<()> {
    type Change = fn(&mut MemoryStake<char>);
    let changes: [Change; 6] = [
        |stake| stake.stake_bond(&'z', &'e', 50).unwrap(),
        |stake| stake.stake_unbond(&'x', 'a', 15).unwrap(),
        |stake| {
            stake.stake_forfeit(&'a', &5_000);
        },
        |stake| stake.stake_exclude(&'a', 99),
        |stake| stake.stake_snapshot_take(),
        |stake| close(stake),
    ];
    for lag in [1, 2] {
        for change in changes {
            let mut stake = rotating(lag);
            stake.wallet_fund(&'z', 100);
            stake.check_weights_settled(change)?;
        }
    }
    let mut stake = delegated(Policy::default());
    close(&mut stake);
    stake.check_weights_settled(|stake| stake.stake_delegate(&'x', Some('c')))?;
    stake.check_weights_settled(|stake| stake.wallet_fund(&'y', 1_000))
}

#[test]
fn rotation_is_the_difference_between_epoch_weights() -> Result<()> {
    for lag in [1, 2] {
        let stake = rotating(lag);
        let schedule = stake.stake_schedule();
        let epochs = 0..6;
        let quorums = subsets(&['a', 'b', 'c', 'd', 'e']);
        for first in epochs.clone() {
            schedule.check_epoch_committee(&first)?;
            schedule.check_rotation_identity(&first)?;
            for second in epochs.clone() {
                schedule.check_rotation_reconstructs(&first, &second)?;
                schedule.check_rotation_inverse(&first, &second)?;
                for third in epochs.clone() {
                    schedule.check_turnover_distance(&first, &second, &third)?;
                }
                for outgoing in &quorums {
                    for incoming in &quorums {
                        schedule.check_handover_overlap(&TwoThirdsQuorum, &first, &second, outgoing, incoming)?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Builds epochs through the churn queue in which a heavy validator enters, stake leaves, and a
/// validator is excluded, checking every snapshot the queue admits.
fn churning() -> Result<MemoryStake<char>> {
    type Change = fn(&mut MemoryStake<char>);
    let changes: [Change; 4] = [
        |_| {},
        |stake| {
            stake.wallet_fund(&'z', 200);
            stake.stake_bond(&'z', &'d', 200).unwrap();
        },
        |stake| stake.stake_unbond(&'x', 'a', 30).unwrap(),
        |stake| stake.stake_exclude(&'b', 99),
    ];
    let mut stake = bonded(Policy::default());
    let rest: Change = |_| {};
    for change in changes.into_iter().chain([rest; 26]) {
        change(&mut stake);
        stake.check_snapshot_admit()?;
        stake.epoch_advance();
    }
    Ok(stake)
}

#[test]
fn the_churn_queue_admits_what_the_limit_allows() -> Result<()> {
    let stake = churning()?;
    ensure!(stake.stake_churn_pending() == 0, "the queue never drained");
    (0..1_000).try_for_each(|total| stake.check_churn_limit(&stake, total))?;
    let schedule = stake.stake_schedule();
    for epoch in 1..30 {
        schedule.check_epoch_committee(&epoch)?;
        if schedule.epoch_total(&epoch) > 0 {
            schedule.check_churn(&stake, &stake, &epoch, &(epoch + 1))?;
        }
    }
    Ok(())
}

#[test]
fn the_churn_queue_spreads_a_large_entry_over_epochs() -> Result<()> {
    let stake = churning()?;
    let entered: Vec<u64> = (2..31).map(|epoch| stake.stake_weight(&'d', &epoch)).collect();
    ensure!(entered.windows(2).all(|pair| pair[0] <= pair[1]), "the entry shrank: {entered:?}");
    ensure!(entered[0] < 200 && entered.last() == Some(&200), "the entry was not spread: {entered:?}");
    Ok(())
}

#[test]
fn unbonding_outlasts_the_evidence_of_every_epoch_it_weighed() -> Result<()> {
    let accountable = |policy: Policy| {
        let stake = MemoryStake::<char>::new(policy);
        (0..8).try_for_each(|start| (0..8).try_for_each(|weighed| stake.check_unbonding_answers(&start, &weighed)))
    };
    accountable(Policy::default())?;
    accountable(Policy { lag: 2, unbonding_delay: 3, ..Policy::default() })?;
    ensure!(accountable(Policy { unbonding_delay: 1, ..Policy::default() }).is_err(), "early maturity passed");
    ensure!(accountable(Policy { lag: 2, ..Policy::default() }).is_err(), "a longer lag passed the same delay");
    Ok(())
}

/// Lists every subset of `validators`.
fn subsets(validators: &[char]) -> Vec<Vec<char>> {
    (0..1u32 << validators.len())
        .map(|mask| {
            validators.iter().enumerate().filter(|(index, _)| mask & (1 << index) != 0).map(|(_, v)| *v).collect()
        })
        .collect()
}

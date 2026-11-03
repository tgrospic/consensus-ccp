//! Witnesses safety across a validator handover with a vote protocol whose slots span two epochs.
use anyhow::{Result, ensure};
use ccp_core::{
    Closed, ExpansionChoices, ExpansionCollapseLawsExt, HandoverLawsExt, HistoryCollapse, HistoryExpansion,
};
use ccp_validator::{
    CommitteeRotationExt, CommitteeSchedule, EpochBindingExt, EpochBindingLawsExt, EpochSorts, SlotEpoch,
    TwoThirdsQuorum, ValidatorSorts, WeightedCommitteeExt,
};
use std::collections::BTreeSet;

/// Holds one vote: a slot, the validator casting it, and the candidate it backs.
type Vote = (u8, char, char);

/// Holds every vote cast so far.
type Votes = BTreeSet<Vote>;

/// Names the one validator that votes for both candidates; one unit of weight is each epoch's fault
/// bound.
const FAULTY: char = 'd';

/// Names the candidates of every slot.
const CANDIDATES: [char; 2] = ['x', 'y'];

/// Splits four slots into two epochs; at the boundary `a` leaves and `e` enters, half the weight.
struct Handover;

impl ValidatorSorts for Handover {
    type Validator = char;
    type Weight = u64;
}

impl EpochSorts for Handover {
    type Epoch = u8;
}

impl CommitteeSchedule for Handover {
    fn committee_at(&self, epoch: &u8) -> Vec<(char, u64)> {
        let members = if *epoch == 0 { "abcd" } else { "bcde" };
        members.chars().map(|validator| (validator, 1)).collect()
    }
}

impl SlotEpoch for Handover {
    type Slot = u8;

    fn epoch_of(&self, slot: &u8) -> u8 {
        slot / 2
    }
}

/// Selects the committee a certificate is weighed by.
#[derive(Clone, Copy)]
enum Weighing {
    /// Weighs a certificate by the committee of its slot's epoch.
    OwnEpoch,
    /// Accepts a certificate that is a quorum of any epoch's committee.
    AnyEpoch,
}

/// Decides a slot for the candidate a quorum votes for, optionally only once every earlier slot is
/// decided.
#[derive(Clone, Copy)]
struct Voting {
    weighing: Weighing,
    gated: bool,
}

impl HistoryExpansion<Votes> for Voting {
    type Input = Vote;
    type Error = ();

    fn expand(&self, votes: &Votes, vote: &Vote) -> Result<Votes, ()> {
        let (slot, validator, _) = *vote;
        let repeated = votes.iter().any(|(kept, voter, _)| *kept == slot && *voter == validator);
        if votes.contains(vote) || (repeated && validator != FAULTY) {
            return Err(());
        }
        let mut later = votes.clone();
        later.insert(*vote);
        Ok(later)
    }
}

impl HistoryCollapse<Votes, u8, Closed<char>> for Voting {
    fn collapse_history(&self, votes: &Votes, slot: &u8) -> Option<Closed<char>> {
        if self.gated && (0..*slot).any(|earlier| self.collapse_history(votes, &earlier).is_none()) {
            return None;
        }
        CANDIDATES
            .into_iter()
            .find(|candidate| {
                let backers = votes.iter().filter(|(kept, _, backed)| kept == slot && backed == candidate);
                let backers = backers.map(|(_, validator, _)| *validator).collect::<Vec<_>>();
                match self.weighing {
                    Weighing::OwnEpoch => Handover.slot_quorum(&TwoThirdsQuorum, slot, backers),
                    Weighing::AnyEpoch => (0..2).any(|epoch| {
                        Handover.epoch_committee(&epoch).weighted_quorum(&TwoThirdsQuorum, backers.iter().copied())
                    }),
                }
            })
            .map(Closed::Commit)
    }
}

/// Offers every vote on the given slots by every validator of either epoch.
struct Offer(Vec<u8>);

impl ExpansionChoices<Votes, Vote> for Offer {
    fn expansion_choices(&self, _votes: &Votes) -> Vec<Vote> {
        self.0
            .iter()
            .flat_map(|slot| "abcde".chars().flat_map(move |validator| CANDIDATES.map(|c| (*slot, validator, c))))
            .collect()
    }
}

/// Lists every subset of the validators of either epoch.
fn coalitions() -> Vec<Vec<char>> {
    let validators = ['a', 'b', 'c', 'd', 'e'];
    (0..1u32 << validators.len())
        .map(|mask| {
            validators.iter().enumerate().filter(|(index, _)| mask & (1 << index) != 0).map(|(_, v)| *v).collect()
        })
        .collect()
}

#[test]
fn slots_fall_in_monotone_epochs() -> Result<()> {
    (0..4).try_for_each(|earlier| (0..4).try_for_each(|later| Handover.check_slot_epochs(&earlier, &later)))
}

#[test]
fn certificates_weighed_by_their_own_epoch_are_safe_across_any_turnover() -> Result<()> {
    ensure!(
        !Handover.handover_quorums_intersect(&TwoThirdsQuorum, &0, &1),
        "the handover was expected to turn over too much for quorums across it to intersect"
    );
    let coalitions = coalitions();
    for slot in 0..4 {
        Handover.check_committee_binding(&Handover.slot_committee(&slot), &slot)?;
        for first in &coalitions {
            for second in &coalitions {
                Handover.check_epoch_commit_safety(&TwoThirdsQuorum, (&slot, &'x', first), (&slot, &'y', second))?;
            }
        }
    }
    Ok(())
}

#[test]
fn weighing_by_another_epoch_breaks_agreement() -> Result<()> {
    let votes: Votes = [(0, 'a', 'x'), (0, 'b', 'x'), (0, 'c', 'y'), (0, 'e', 'y')].into();
    let faulty = Offer(vec![0]);
    Voting { weighing: Weighing::OwnEpoch, gated: true }.check_collapse_agreement(&votes, &[0], &faulty, 2)?;
    ensure!(
        Voting { weighing: Weighing::AnyEpoch, gated: true }
            .check_collapse_agreement(&votes, &[0], &faulty, 2)
            .is_err(),
        "a certificate weighed by the next epoch's committee kept agreement"
    );
    Ok(())
}

#[test]
fn decisions_after_a_handover_extend_the_closed_prefix() -> Result<()> {
    let votes: Votes =
        [(0, 'a', 'x'), (0, 'b', 'x'), (0, 'c', 'x'), (1, 'a', 'y'), (1, 'b', 'y'), (2, 'b', 'x'), (2, 'c', 'x')]
            .into();
    let offer = Offer(vec![1, 2]);
    let gated = Voting { weighing: Weighing::OwnEpoch, gated: true };
    gated.check_handover_continuity(&votes, &[0, 1], &[2, 3], &offer, 2)?;
    gated.check_collapse_agreement(&votes, &[0, 1, 2, 3], &offer, 2)?;
    ensure!(
        Voting { weighing: Weighing::OwnEpoch, gated: false }
            .check_handover_continuity(&votes, &[0, 1], &[2, 3], &offer, 2)
            .is_err(),
        "the next epoch decided past an open slot of the previous one"
    );
    Ok(())
}

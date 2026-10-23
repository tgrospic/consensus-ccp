//! Compares protocol leader schedules over one shared legitimacy relation.

use ccp::communication::{MessageCarrier, MessageInput, MessageSlot, QuorumAdvancePolicy, quorum_advance_policy};
use ccp::leader::{LeaderSelection, LeaderSelectionPolicy};
use ccp::protocols::adaptive::adaptive_policy;
use ccp::protocols::cordial::cordial_policy;
use ccp::protocols::dense_leader::dense_leader_policy;
use ccp::protocols::mysticeti::mysticeti_policy;
use ccp::quorum::CommitteeOrder;
use ccp::{Closed, HistoryCollapse};
use ccp_runtime::{
    CandidateId, CommunicationExpansion, CommunicationHistory, CordialCollapse, Machine, MysticetiCollapse,
    ObservedSlot, ScheduledLeaderCollapse, ValidatorId,
};

type History = CommunicationHistory<ValidatorId, CandidateId>;
type Expansion = CommunicationExpansion<QuorumAdvancePolicy>;
type ComparisonMachine = Machine<History, Expansion, ()>;

/// Runs this comparison and prints its observations.
pub(crate) fn run() {
    let history = CommunicationHistory::genesis((0..4).map(ValidatorId).collect()).expect("valid committee");
    let mut machine = Machine::new(history, CommunicationExpansion::new(quorum_advance_policy()), ());
    add_history(&mut machine);

    println!("reachable communication history");
    println!("legend: x=produced message, C=committed leader, -=skipped leader");
    println!("        ?=unresolved leader, ~=missing unresolved leader, .=produced non-leader\n");
    print_production(machine.history());

    let dense = dense_leader_policy();
    let mysticeti = mysticeti_policy(2);
    let adaptive = adaptive_policy(3);
    let cordial = cordial_policy(0, 3);
    let dense_collapse = ScheduledLeaderCollapse::new(dense.clone());
    let mysticeti_collapse = MysticetiCollapse::new(mysticeti.clone());
    let adaptive_collapse = ScheduledLeaderCollapse::new(adaptive.clone());
    let cordial_collapse = CordialCollapse::new(cordial.clone());
    print_collapse("dense leader collapse", machine.history(), &dense, |slot| {
        dense_collapse.collapse_history(machine.history(), &ObservedSlot::new(*slot, CandidateId(40)))
    });
    print_collapse("Mysticeti collapse", machine.history(), &mysticeti, |slot| {
        mysticeti_collapse.collapse_history(machine.history(), slot)
    });
    print_collapse("adaptive collapse", machine.history(), &adaptive, |slot| {
        adaptive_collapse.collapse_history(machine.history(), &ObservedSlot::new(*slot, CandidateId(40)))
    });
    print_collapse("Cordial collapse", machine.history(), &cordial, |slot| {
        cordial_collapse.collapse_history(machine.history(), &ObservedSlot::new(*slot, CandidateId(40)))
    });
}

fn add_history(machine: &mut ComparisonMachine) {
    let mut previous = Vec::new();
    for round in 0..5 {
        let mut current = Vec::new();
        let produced = if round == 4 { 1 } else { 3 };
        for validator in 0..produced {
            let candidate = CandidateId(round * 10 + validator);
            machine
                .apply(&MessageInput::new(MessageSlot::new(round, ValidatorId(validator)), candidate, previous.clone()))
                .expect("three connected validators advance every round");
            current.push(candidate);
        }
        previous = current;
    }
}

fn print_production(history: &History) {
    println!("production");
    println!("round       |  0  1  2  3  4");
    for validator in 0..4 {
        let cells =
            format_cells((0..5).map(|round| {
                history.message_at(&MessageSlot::new(round, ValidatorId(validator))).map_or(' ', |_| 'x')
            }));
        println!("validator {validator} |{cells}");
    }
    println!();
}

fn print_collapse<Schedule, Decide>(name: &str, history: &History, schedule: &Schedule, decide: Decide)
where
    Schedule: LeaderSelectionPolicy<u64, ValidatorId>,
    Decide: Fn(&MessageSlot<ValidatorId>) -> Option<Closed<CandidateId>>,
{
    let committee = history.committee_order();
    println!("{name}");
    println!("observer: 40 at round 4, validator 0");
    println!("round       |  0  1  2  3  4");
    for validator in 0..4 {
        let cells = format_cells((0..5).map(|round| {
            let slot = MessageSlot::new(round, ValidatorId(validator));
            let produced = history.message_at(&slot).is_some();
            let selected = matches!(
                schedule.select_leaders(&round, &committee),
                LeaderSelection::Selected(leaders) if leaders.contains(&ValidatorId(validator))
            );
            match decide(&slot) {
                Some(Closed::Commit(_)) => 'C',
                Some(Closed::Skip) => '-',
                None if selected && produced => '?',
                None if selected => '~',
                None if produced => '.',
                None => ' ',
            }
        }));
        println!("validator {validator} |{cells}");
    }
    println!();
}

fn format_cells(cells: impl Iterator<Item = char>) -> String {
    cells.flat_map(|cell| [' ', ' ', cell]).collect()
}

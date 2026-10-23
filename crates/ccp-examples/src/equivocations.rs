//! Shows that lawful expansion retains an equivocation without changing canonical projection.

use ccp::communication::{MessageCarrierExt, MessageInput, MessageSlot, QuorumAdvancePolicy, quorum_advance_policy};
use ccp_runtime::{CandidateId, CommunicationExpansion, CommunicationHistory, Machine, ValidatorId};

/// Runs this comparison and prints its observations.
pub(crate) fn run() {
    let history = CommunicationHistory::genesis((0..4).map(ValidatorId).collect()).expect("valid committee");
    let mut machine = Machine::new(history, CommunicationExpansion::new(quorum_advance_policy()), ());

    add_round(&mut machine, 0, &[]);
    add_round(&mut machine, 1, &[0, 1, 2]);
    machine
        .apply(&MessageInput::new(
            MessageSlot::new(1, ValidatorId(0)),
            CandidateId(100),
            vec![CandidateId(0), CandidateId(1), CandidateId(2)],
        ))
        .expect("one Byzantine equivocator is retained");

    println!("one lawful history, two projections");
    println!("legend: a/b = canonical candidate plus retained equivocation, blank = no message\n");
    print_history(machine.history());
}

fn add_round(
    machine: &mut Machine<
        CommunicationHistory<ValidatorId, CandidateId>,
        CommunicationExpansion<QuorumAdvancePolicy>,
        (),
    >,
    round: u64,
    justifications: &[u64],
) {
    for validator in 0..3 {
        machine
            .apply(&MessageInput::new(
                MessageSlot::new(round, ValidatorId(validator)),
                CandidateId(round * 10 + validator),
                justifications.iter().copied().map(CandidateId).collect(),
            ))
            .expect("generated round is lawful");
    }
}

fn print_history(history: &CommunicationHistory<ValidatorId, CandidateId>) {
    println!("round       |{:>8}{:>8}", 0, 1);
    for validator in 0..4 {
        let cells = (0..=1)
            .map(|round| format_cell(history, &MessageSlot::new(round, ValidatorId(validator))))
            .collect::<Vec<_>>()
            .join(" ");
        println!("validator {validator} | {cells}");
    }
}

fn format_cell(history: &CommunicationHistory<ValidatorId, CandidateId>, slot: &MessageSlot<ValidatorId>) -> String {
    let candidates = history.slot_messages(slot);
    if candidates.is_empty() {
        "       ".to_owned()
    } else {
        format!("{:>7}", candidates.iter().map(ToString::to_string).collect::<Vec<_>>().join("/"))
    }
}

//! Shows proof-of-work expansion, fork choice, and confirmation-depth collapse.

use ccp::protocols::nakamoto::{NakamotoPolicy, NakamotoRiskExt, NakamotoRiskModel};
use ccp_runtime::{CandidateId, Machine, NakamotoCollapse, WorkBlockInput, WorkExpansion, WorkHistory};

/// Runs this comparison and prints its observations.
pub(crate) fn run() {
    let collapse = NakamotoCollapse::new(NakamotoPolicy::new(2));
    let mut machine = Machine::new(WorkHistory::default(), WorkExpansion, collapse);

    add(&mut machine, 0, None, 1);
    add(&mut machine, 1, Some(0), 1);
    add(&mut machine, 2, Some(1), 1);
    add(&mut machine, 10, Some(0), 1);
    add(&mut machine, 11, Some(10), 1);
    println!("{:<22}{:?}", "equal-work branches:", machine.collapse(&()));

    add(&mut machine, 12, Some(11), 2);
    let prefix = machine.collapse(&()).expect("additional work selects one branch");
    println!("{:<22}{}", "selected tip:", prefix.tip);
    println!("{:<22}{}", "accepted prefix:", format_candidates(&prefix.accepted));
    println!("{:<22}{}", "tentative suffix:", format_candidates(&prefix.tentative));
    println!("{:<22}{:?}", "reversal-risk shape:", NakamotoRiskModel::new(1, 3).reversal_risk_bound(2));
}

fn format_candidates(candidates: &[CandidateId]) -> String {
    format!("[{}]", candidates.iter().map(ToString::to_string).collect::<Vec<_>>().join(", "))
}

fn add(
    machine: &mut Machine<WorkHistory<CandidateId>, WorkExpansion, NakamotoCollapse>,
    block: u64,
    parent: Option<u64>,
    work: u128,
) {
    machine
        .apply(&WorkBlockInput::new(CandidateId(block), parent.map(CandidateId), work))
        .expect("example block is a lawful expansion");
}

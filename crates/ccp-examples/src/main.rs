//! Runs executable CCP protocol comparisons through the constructive machine.

mod equivocations;
mod nakamoto;
mod policies;

/// Runs the comparison named by the first argument, or every comparison when none is named.
fn main() {
    match std::env::args().nth(1).as_deref() {
        None => {
            policies::run();
            equivocations::run();
            nakamoto::run();
        }
        Some("policies") => policies::run(),
        Some("equivocations") => equivocations::run(),
        Some("nakamoto") => nakamoto::run(),
        Some(other) => {
            eprintln!("unknown comparison `{other}`; expected policies, equivocations, or nakamoto");
            std::process::exit(2);
        }
    }
}

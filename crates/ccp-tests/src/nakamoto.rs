//! Runs the Nakamoto, confidence, and candidate-space laws on reachable proof-of-work histories.
use anyhow::{Result, anyhow, ensure};
use ccp_confidence::ConfidenceLawsExt;
use ccp_core::CandidateSpaceLawsExt;
use ccp_nakamoto::{
    ConfirmationConfidence, NakamotoCollapseExt, NakamotoConfirmation, NakamotoLawsExt, NakamotoPolicy,
    NakamotoRiskLawsExt, NakamotoRiskModel,
};
use ccp_runtime::{CandidateId, Machine, WorkBlockInput, WorkExpansion, WorkHistory};

#[test]
fn proof_of_work_histories_satisfy_nakamoto_laws() -> Result<()> {
    let policies = [0, 1, 2, 3].map(NakamotoPolicy::new);
    let blocks =
        [(0, None, 1), (1, Some(0), 1), (2, Some(1), 1), (10, Some(0), 1), (11, Some(10), 1), (12, Some(11), 2)];
    let mut machine = Machine::new(WorkHistory::default(), WorkExpansion, ());
    blocks.into_iter().try_for_each(|(block, parent, work)| {
        let input = WorkBlockInput::new(CandidateId(block), parent.map(CandidateId), work);
        machine.apply(&input).map_err(|error| anyhow!("lawful block was rejected: {error:?}"))?;
        let history = machine.history();
        history.check_candidate_space()?;
        history.check_nakamoto_tree()?;
        policies.iter().try_for_each(|policy| history.check_nakamoto_prefix(policy))
    })
}

#[test]
fn confirmation_policies_accept_upward_closed_confidence() -> Result<()> {
    let confidences = (0..6).map(ConfirmationConfidence).collect::<Vec<_>>();
    (0..5)
        .try_for_each(|depth| NakamotoPolicy::new(depth).check_confidence_threshold(NakamotoConfirmation, &confidences))
}

#[test]
fn risk_models_satisfy_hash_fraction_laws() -> Result<()> {
    [(0, 1), (1, 3), (1, 2), (2, 5)].into_iter().try_for_each(|(numerator, denominator)| {
        NakamotoRiskModel::new(numerator, denominator).check_nakamoto_risk_model()
    })
}

/// Applies blocks as `(block, parent, work)` and returns the accepted prefix after each.
fn accepted_after(blocks: &[(u64, Option<u64>, u128)], policy: NakamotoPolicy) -> Result<Vec<Vec<CandidateId>>> {
    let mut machine = Machine::new(WorkHistory::default(), WorkExpansion, ());
    blocks
        .iter()
        .map(|(block, parent, work)| {
            let input = WorkBlockInput::new(CandidateId(*block), parent.map(CandidateId), *work);
            machine.apply(&input).map_err(|error| anyhow!("lawful block was rejected: {error:?}"))?;
            Ok(machine.history().nakamoto_prefix(&policy).map(|prefix| prefix.accepted).unwrap_or_default())
        })
        .collect()
}

#[test]
fn accepted_prefixes_are_safe_only_against_less_work() -> Result<()> {
    let policy = NakamotoPolicy::new(1);
    let honest = [(0, None, 1), (1, Some(0), 1), (2, Some(1), 1), (3, Some(2), 1), (4, Some(3), 1)];
    let accepted = accepted_after(&honest, policy)?;
    ensure!(
        accepted.windows(2).all(|pair| pair[1].starts_with(&pair[0])),
        "extending the heaviest chain reversed an accepted block: {accepted:?}"
    );
    let forked = [&honest[..3], &[(10, Some(0), 2), (11, Some(10), 2)]].concat();
    let accepted = accepted_after(&forked, policy)?;
    ensure!(
        !accepted[accepted.len() - 1].starts_with(&accepted[2]),
        "a heavier fork left the accepted prefix in place: {accepted:?}"
    );
    Ok(())
}

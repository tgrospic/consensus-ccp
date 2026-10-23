//! Runs the Nakamoto, confidence, and candidate-space laws on reachable proof-of-work histories.
use anyhow::{Result, anyhow};
use ccp_confidence::ConfidenceLawsExt;
use ccp_core::CandidateSpaceLawsExt;
use ccp_nakamoto::{
    ConfirmationConfidence, NakamotoConfirmation, NakamotoLawsExt, NakamotoPolicy, NakamotoRiskLawsExt,
    NakamotoRiskModel,
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

//! Runs the specifications' laws on every history a conformance scenario reaches.
use crate::scenario::{CollapseExpectation, History, Input, expansion};
use anyhow::Result;
use ccp_communication::{
    ExpansionError, JustificationRelation, MessageCarrier, MessageCarrierExt, MessageSlot, ObservedRelationEvidence,
};
use ccp_cordial::{CordialCollapseExt, CordialLawsExt, CordialPolicy};
use ccp_core::{
    CandidateRestriction, CandidateSpaceLawsExt, Closed, ClosedPrefixLawsExt, Decision, EvidenceHorizon,
    ExpansionChoices, ExpansionCollapseLawsExt, HistoryCollapse, HistoryExpansion, RestrictionRefinementLawsExt,
};
use ccp_dense_leader::DenseLeaderLawsExt;
use ccp_leader::{AllLeaderSlots, CandidateAuthorLawsExt, LeaderSelectionPolicy, Policy, reference_policy};
use ccp_mysticeti::{
    MysticetiCollapseViewExt, MysticetiDagEvidence, MysticetiLawsExt, MysticetiPolicy, MysticetiRepairLawsExt,
};
use ccp_runtime::{
    CandidateId, CordialCollapse, CordialDagView, DenseLeaderCollapse, MysticetiCollapse, ObservedSlot,
    ScheduledHistory, ScheduledLeaderCollapse, ScheduledLeaderView, ValidatorId,
};

/// Selects the protocol whose laws a scenario's collapse expectation brings into scope.
#[derive(Debug, Clone)]
pub(crate) enum ProtocolLaws {
    Dense,
    Scheduled(MysticetiPolicy),
    ScheduledCordial(CordialPolicy),
    Cordial(CordialPolicy),
    Mysticeti(MysticetiPolicy),
}

impl ProtocolLaws {
    pub(crate) fn of(expectation: &CollapseExpectation) -> Self {
        match expectation {
            CollapseExpectation::Dense(_) => Self::Dense,
            CollapseExpectation::Scheduled(policy, _) => Self::Scheduled(policy.clone()),
            CollapseExpectation::ScheduledCordial(policy, _) => Self::ScheduledCordial(policy.clone()),
            CollapseExpectation::Cordial(policy, _) => Self::Cordial(policy.clone()),
            CollapseExpectation::Mysticeti(policy, _) => Self::Mysticeti(policy.clone()),
        }
    }

    /// Checks the restriction and protocol-view laws at one history.
    pub(crate) fn check_history(&self, history: &History) -> Result<()> {
        match self {
            Self::Dense => check_dense_view(history, &AllLeaderSlots),
            Self::Scheduled(policy) => {
                check_policy_refinement(history, policy)?;
                check_dense_view(history, policy)
            }
            Self::ScheduledCordial(policy) => {
                check_policy_refinement(history, policy)?;
                check_dense_view(history, policy)
            }
            Self::Cordial(policy) => {
                check_policy_refinement(history, policy)?;
                check_cordial_view(history, policy)
            }
            Self::Mysticeti(policy) => {
                check_policy_refinement(history, policy)?;
                check_mysticeti_view(history, policy)
            }
        }
    }

    /// Checks that collapse and closed prefixes survive one accepted expansion.
    pub(crate) fn check_expansion(&self, earlier: &History, later: &History, input: &Input) -> Result<()> {
        match self {
            Self::Dense => Interpreted(DenseLeaderCollapse).check_observed_expansion(earlier, later, input),
            Self::Scheduled(policy) => Interpreted(ScheduledLeaderCollapse::new(policy.clone()))
                .check_observed_expansion(earlier, later, input),
            Self::ScheduledCordial(policy) => Interpreted(ScheduledLeaderCollapse::new(policy.clone()))
                .check_observed_expansion(earlier, later, input),
            Self::Cordial(policy) => {
                Interpreted(CordialCollapse::new(policy.clone())).check_observed_expansion(earlier, later, input)?;
                observers(earlier).iter().try_for_each(|observer| {
                    CordialDagView::new(earlier, policy, observer)
                        .cordial_closed_prefix()
                        .check_closed_prefix_extension(
                            &CordialDagView::new(later, policy, observer).cordial_closed_prefix(),
                        )
                })
            }
            Self::Mysticeti(policy) => {
                let slots = earlier.message_slots();
                let interpreted = Interpreted(MysticetiCollapse::new(policy.clone()));
                interpreted.check_history_collapse_expansion(earlier, input, &slots)?;
                interpreted.check_collapse_monotonicity(earlier, later, &slots, &OneInput(input.clone()), 1)?;
                mysticeti_closed_prefix(earlier, policy)
                    .check_closed_prefix_extension(&mysticeti_closed_prefix(later, policy))
            }
        }
    }
}

/// Checks the candidate-space and authorship laws every communication history satisfies.
pub(crate) fn check_carrier_laws(history: &History) -> Result<()> {
    history.check_candidate_space()?;
    history.check_candidate_authors()
}

/// Pairs the scenario's constructive expansion with one protocol collapse.
struct Interpreted<Collapse>(Collapse);

/// Offers exactly one next input to bounded exploration.
struct OneInput(Input);

impl<Collapse> HistoryExpansion<History> for Interpreted<Collapse> {
    type Input = Input;
    type Error = ExpansionError;

    fn expand(&self, history: &History, input: &Input) -> Result<History, ExpansionError> {
        expansion().expand(history, input)
    }
}

impl<Collapse, Q, P> HistoryCollapse<History, Q, P> for Interpreted<Collapse>
where
    Collapse: HistoryCollapse<History, Q, P>,
{
    fn collapse_history(&self, history: &History, query: &Q) -> Option<P> {
        self.0.collapse_history(history, query)
    }
}

impl ExpansionChoices<History, Input> for OneInput {
    fn expansion_choices(&self, _history: &History) -> Vec<Input> {
        vec![self.0.clone()]
    }
}

impl<Collapse> Interpreted<Collapse>
where
    Collapse: HistoryCollapse<History, ObservedSlot<ValidatorId, CandidateId>, Closed<CandidateId>>,
{
    fn check_observed_expansion(&self, earlier: &History, later: &History, input: &Input) -> Result<()> {
        let queries = earlier
            .message_slots()
            .into_iter()
            .flat_map(|slot| observers(earlier).into_iter().map(move |observer| ObservedSlot::new(slot, observer)))
            .collect::<Vec<_>>();
        self.check_history_collapse_expansion(earlier, input, &queries)?;
        self.check_collapse_monotonicity(earlier, later, &queries, &OneInput(input.clone()), 1)
    }
}

fn observers(history: &History) -> Vec<CandidateId> {
    history.messages()
}

fn check_policy_refinement<Candidates, Horizon, Leaders>(
    history: &History,
    policy: &Policy<Candidates, Horizon, Leaders>,
) -> Result<()>
where
    Policy<Candidates, Horizon, Leaders>:
        CandidateRestriction<MessageSlot<ValidatorId>, CandidateId> + EvidenceHorizon<MessageSlot<ValidatorId>>,
{
    reference_policy().check_restriction_refinement(policy, history)
}

fn check_dense_view<Schedule>(history: &History, schedule: &Schedule) -> Result<()>
where
    Schedule: LeaderSelectionPolicy<u64, ValidatorId>,
{
    let view = ScheduledLeaderView::new(history, schedule);
    observers(history).iter().try_for_each(|observer| {
        let evidence = ObservedRelationEvidence::new(history, observer, JustificationRelation::new(history));
        history
            .message_slots()
            .iter()
            .try_for_each(|frontier| view.check_dense_leader_finalizations(&evidence, frontier))
    })
}

fn check_cordial_view(history: &History, policy: &CordialPolicy) -> Result<()> {
    observers(history).iter().try_for_each(|observer| {
        let view = CordialDagView::new(history, policy, observer);
        view.check_cordial_anchor_chain()?;
        history.message_slots().iter().try_for_each(|frontier| view.check_cordial_frontier_suffix(frontier))?;
        view.check_cordial_fragments()
    })
}

fn check_mysticeti_view(history: &History, policy: &MysticetiPolicy) -> Result<()> {
    let view = ScheduledHistory::new(history, policy);
    view.check_mysticeti_direct_exclusivity(&MysticetiDagEvidence::new(history, policy))?;
    view.check_mysticeti_repair_order()
}

fn mysticeti_closed_prefix(
    history: &History,
    policy: &MysticetiPolicy,
) -> Vec<Decision<MessageSlot<ValidatorId>, CandidateId>> {
    ScheduledHistory::new(history, policy).mysticeti_closed_prefix(&MysticetiDagEvidence::new(history, policy))
}

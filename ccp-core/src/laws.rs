use crate::{
    CandidateRestriction, CandidateRestrictionExt, CandidateSpace, Closed, CollapsePolicyExt, CollapseRule, Decision,
    Denotation, EvidenceDenotation, EvidenceExtraction, EvidenceHorizon, EvidenceHorizonExt, ExpansionChoices,
    ExpansionCollapseExt, FiniteExpansionExt, HistoryCollapse, HistoryDenotation, HistoryExpansion, HorizonProjection,
    OutputDenotation, PredecessorChainExt, ReferenceDecisionExt, SemanticClosure, VisibleDenotation,
};
use alux_ext::ext;
use anyhow::{Result, anyhow, bail, ensure};
use std::{collections::HashSet, fmt::Debug, hash::Hash};

/// Checks finite predecessor closure independently from its carrier representation.
#[ext(name = PredecessorChainLawsExt)]
pub impl<This, C> This
where
    This: Fn(&C) -> Option<C>,
    C: Clone + Eq + Hash,
{
    /// Checks anchor retention, uniqueness, adjacency, and exhaustion or cycle closure.
    fn check_predecessor_chain(&self, anchor: C) -> Result<()> {
        let chain = self.predecessor_chain(anchor.clone());
        ensure!(chain.last() == Some(&anchor), "predecessor chain loses its anchor");
        let members = chain.iter().collect::<HashSet<_>>();
        ensure!(members.len() == chain.len(), "predecessor chain repeats a candidate");
        ensure!(
            chain.windows(2).all(|pair| self(&pair[1]).as_ref() == Some(&pair[0])),
            "predecessor chain breaks adjacency"
        );
        if let Some(previous) = chain.first().and_then(self) {
            ensure!(members.contains(&previous), "predecessor chain stops before closure");
        }
        Ok(())
    }
}

/// Checks candidate-space functionality independently from its representation.
#[ext(name = CandidateSpaceLawsExt)]
pub impl<This> This
where
    This: CandidateSpace,
{
    /// Checks ordered slots, unique candidates, and candidate-to-slot round trips.
    fn check_candidate_space(&self) -> Result<()>
    where
        This::Slot: Clone + Ord + Debug,
        This::Candidate: Clone + Eq + Hash + Debug,
    {
        let slots = self.slots();
        ensure!(slots.windows(2).all(|pair| pair[0] < pair[1]), "candidate slots are not strictly ordered");
        let candidates = slots
            .iter()
            .flat_map(|slot| self.candidates(slot).into_iter().map(move |candidate| (slot, candidate)))
            .collect::<Vec<_>>();
        ensure!(
            candidates.iter().map(|(_, candidate)| candidate).collect::<HashSet<_>>().len() == candidates.len(),
            "a candidate occupies more than one slot"
        );
        candidates.into_iter().try_for_each(|(slot, candidate)| {
            ensure!(
                self.slot_of(&candidate) == *slot,
                "candidate-to-slot interpretation does not round trip: {candidate:?}"
            );
            Ok(())
        })
    }
}

/// Checks the paper's concrete-to-denotational realization squares.
#[ext(name = RealizationLawsExt)]
pub impl<This> This {
    /// Checks legitimacy, horizon, collapse, and induced-decision preservation.
    fn check_realization<Concrete, Specification>(
        &self,
        concrete: &Concrete,
        specification: &Specification,
        history: &Concrete::History,
    ) -> Result<()>
    where
        Concrete: EvidenceExtraction + HorizonProjection + CollapseRule,
        Specification: EvidenceExtraction + HorizonProjection + CollapseRule,
        This: Denotation<HistoryDenotation, Concrete::History, Specification::History>
            + Denotation<EvidenceDenotation, Concrete::Evidence, Specification::Evidence>
            + Denotation<VisibleDenotation, Concrete::Visible, Specification::Visible>
            + Denotation<OutputDenotation, Concrete::Output, Specification::Output>,
        Specification::Evidence: Eq,
        Specification::Visible: Eq,
        Specification::Output: Eq,
    {
        let concrete_evidence = concrete.extract_evidence(history);
        let denoted_history = self.denote(&HistoryDenotation, history);
        ensure!(
            self.denote(&EvidenceDenotation, &concrete_evidence) == specification.extract_evidence(&denoted_history),
            "evidence extraction does not preserve its denotation"
        );

        let concrete_visible = concrete.project_horizon(&concrete_evidence);
        let denoted_evidence = self.denote(&EvidenceDenotation, &concrete_evidence);
        ensure!(
            self.denote(&VisibleDenotation, &concrete_visible) == specification.project_horizon(&denoted_evidence),
            "horizon projection does not preserve its denotation"
        );

        if let Some(concrete_output) = concrete.collapse(&concrete_visible) {
            let denoted_visible = self.denote(&VisibleDenotation, &concrete_visible);
            ensure!(
                Some(self.denote(&OutputDenotation, &concrete_output)) == specification.collapse(&denoted_visible),
                "collapse does not preserve its denotation"
            );
        }

        if let Some(concrete_output) = concrete.collapse_policy(history) {
            ensure!(
                Some(self.denote(&OutputDenotation, &concrete_output))
                    == specification.collapse_policy(&denoted_history),
                "induced decision map does not preserve its denotation"
            );
        }

        Ok(())
    }
}

/// Checks collapse stability across lawful communication expansion.
#[ext(name = ExpansionCollapseLawsExt)]
pub impl<This, H, S, C> This
where
    This: HistoryExpansion<H> + HistoryCollapse<H, S, Closed<C>>,
    H: Clone + Eq,
    C: Clone + Eq,
{
    /// Checks that bounded invariant collapse remains identical at a reachable later history.
    fn check_collapse_monotonicity<Choices>(
        &self,
        earlier: &H,
        later: &H,
        slots: &[S],
        choices: &Choices,
        max_steps: usize,
    ) -> Result<()>
    where
        Choices: ExpansionChoices<H, This::Input>,
        S: Eq + Debug,
        C: Debug,
    {
        ensure!(
            self.expansion_reaches(earlier, later, choices, max_steps),
            "later history is not a lawful expansion of the earlier history"
        );
        slots.iter().try_for_each(|slot| {
            if let Some(closed) = self.bounded_invariant_collapse(earlier, slot, choices, max_steps) {
                ensure!(
                    self.bounded_invariant_collapse(later, slot, choices, max_steps).as_ref() == Some(&closed),
                    "collapse changed under lawful expansion at {slot:?}: {closed:?}"
                );
            }
            Ok(())
        })
    }

    /// Checks that a produced history collapse is preserved by expansion.
    fn check_history_collapse_expansion(&self, earlier: &H, input: &This::Input, slots: &[S]) -> Result<()>
    where
        S: Debug,
        C: Debug,
        This::Error: Debug,
    {
        let later = self.expand(earlier, input).map_err(|error| anyhow!("history expansion failed: {error:?}"))?;
        slots.iter().try_for_each(|slot| {
            if let Some(observation) = self.collapse_history(earlier, slot) {
                ensure!(
                    self.collapse_history(&later, slot).as_ref() == Some(&observation),
                    "history collapse changed under expansion at {slot:?}: {observation:?}"
                );
            }
            Ok(())
        })
    }
}

/// Checks refinement of closed protocol prefixes.
#[ext(name = ClosedPrefixLawsExt)]
pub impl<This, S, C> This
where
    This: AsRef<[Decision<S, C>]>,
{
    /// Checks that `later` extends this closed prefix without changing an earlier decision.
    fn check_closed_prefix_extension<Later>(&self, later: &Later) -> Result<()>
    where
        Later: AsRef<[Decision<S, C>]>,
        S: Eq + Debug,
        C: Eq + Debug,
    {
        let earlier = self.as_ref();
        let later = later.as_ref();
        ensure!(earlier.iter().all(Decision::is_closed), "earlier prefix contains an undecided slot");
        ensure!(later.iter().all(Decision::is_closed), "later prefix contains an undecided slot");
        ensure!(earlier.len() <= later.len(), "later prefix is shorter than the earlier prefix");
        ensure!(
            earlier.iter().zip(later).all(|(earlier, later)| earlier == later),
            "later prefix changes an earlier decision"
        );
        Ok(())
    }
}

/// Checks refinement between two candidate restrictions.
#[ext(name = CandidateRestrictionLawsExt)]
pub impl<This, S, C> This
where
    This: CandidateRestriction<S, C>,
{
    /// Checks that `restricted` only removes candidates admitted by `self`.
    fn check_candidate_refinement<Restricted, Space>(&self, restricted: &Restricted, carrier: &Space) -> Result<()>
    where
        Restricted: CandidateRestriction<S, C>,
        Space: CandidateSpace<Slot = S, Candidate = C>,
        S: Debug,
        C: Clone + Eq + Debug,
    {
        carrier.slots().into_iter().try_for_each(|slot| {
            let wider = self.eligible_candidates(carrier, &slot);
            restricted.eligible_candidates(carrier, &slot).into_iter().try_for_each(|candidate| {
                ensure!(
                    wider.contains(&candidate),
                    "candidate restriction is not a refinement at {slot:?}: {candidate:?}"
                );
                Ok(())
            })
        })
    }
}

/// Checks refinement between two evidence horizons.
#[ext(name = EvidenceHorizonLawsExt)]
pub impl<This, S> This
where
    This: EvidenceHorizon<S>,
{
    /// Checks that `restricted` exposes no evidence slot hidden by `self`.
    fn check_horizon_refinement<Restricted, Space, C>(
        &self,
        restricted: &Restricted,
        carrier: &Space,
        candidate_slot: &S,
    ) -> Result<()>
    where
        Restricted: EvidenceHorizon<S>,
        Space: CandidateSpace<Slot = S, Candidate = C>,
        S: Clone + Eq + Debug,
    {
        let wider = self.visible_slots(carrier, candidate_slot);
        restricted.visible_slots(carrier, candidate_slot).into_iter().try_for_each(|slot| {
            ensure!(wider.contains(&slot), "evidence horizon is not a refinement: exposed {slot:?}");
            Ok(())
        })
    }
}

/// Checks refinement between two restriction policies over both candidates and evidence.
#[ext(name = RestrictionRefinementLawsExt)]
pub impl<This, S, C> This
where
    This: CandidateRestriction<S, C> + EvidenceHorizon<S>,
{
    /// Checks that `restricted` only removes candidates and only hides evidence relative to this
    /// policy, at every slot of `carrier`.
    fn check_restriction_refinement<Restricted, Space>(&self, restricted: &Restricted, carrier: &Space) -> Result<()>
    where
        Restricted: CandidateRestriction<S, C> + EvidenceHorizon<S>,
        Space: CandidateSpace<Slot = S, Candidate = C>,
        S: Clone + Eq + Debug,
        C: Clone + Eq + Debug,
    {
        self.check_candidate_refinement(restricted, carrier)?;
        carrier.slots().iter().try_for_each(|slot| self.check_horizon_refinement(restricted, carrier, slot))
    }
}

/// Checks protocol decisions against unrestricted semantic closure.
#[ext(name = CollapseRefinementLawsExt)]
pub impl<This, S, C> This
where
    This: SemanticClosure<Slot = S, Candidate = C>,
{
    /// Checks that a closed restricted decision agrees with unrestricted semantic closure.
    fn check_decision_refinement(&self, slot: &S, decision: &Decision<S, C>) -> Result<()>
    where
        S: PartialEq + Clone + Debug,
        C: Clone + Eq + Debug,
    {
        let reference = self.reference_decision(slot);
        if decision.is_closed() && &reference != decision {
            bail!(
                "restricted collapse disagrees with reference closure at {slot:?}: \
                 reference={reference:?}, restricted={decision:?}"
            );
        }
        Ok(())
    }
}

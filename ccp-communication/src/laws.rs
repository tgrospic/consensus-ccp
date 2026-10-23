use crate::{
    AdvancePolicy, AdvancePolicyExt, ExpansionError, MessageAdmissionExt, MessageCarrier, MessageCarrierExt,
    MessageInput, MessageJustification, MessageJustificationExt, MessageSlot,
};
use alux_ext::ext;
use anyhow::{Result, anyhow, bail, ensure};
use ccp_core::HistoryExpansion;
use ccp_quorum::{CommitteeLawsExt, CommitteeOrder, QuorumLawsExt, QuorumThreshold, ValidatorSet};
use std::{collections::HashSet, fmt::Debug, hash::Hash};

/// Checks that a message history is lawful under a round-advance policy.
#[ext(name = MessageHistoryLawsExt)]
pub impl<This, V, C> This
where
    This: CommitteeOrder<Validator = V> + MessageCarrier<Message = C> + MessageJustification + ValidatorSet,
{
    /// Checks identity, reference, causal-order, equivocation, and round-advance obligations.
    fn check_lawful_message_history<Policy>(&self, advance: &Policy) -> Result<()>
    where
        Policy: AdvancePolicy + QuorumThreshold,
        V: Clone + Eq + Hash + Debug,
        C: Clone + Eq + Hash + Debug,
    {
        self.check_committee_order()?;
        advance.check_quorum_threshold(self.validators().len())?;
        ensure!(advance.window() != 0, "advance window is zero");

        let messages = self.messages();
        ensure!(
            messages.iter().collect::<HashSet<_>>().len() == messages.len(),
            "a message occupies more than one communication slot"
        );
        messages.iter().try_for_each(|message| {
            let source = self.message_slot_of(message).expect("message uniqueness was checked");
            self.message_justifications(message).into_iter().try_for_each(|target| {
                let target_slot = self
                    .message_slot_of(&target)
                    .ok_or_else(|| anyhow!("message {message:?} justifies unknown target {target:?}"))?;
                ensure!(
                    target_slot.round < source.round,
                    "message justification does not point to an earlier round: {message:?} -> {target:?}"
                );
                Ok(())
            })
        })?;

        let rounds = self.message_slots().into_iter().map(|slot| slot.round).collect::<HashSet<_>>();
        ensure!(
            rounds.iter().all(|round| advance.advances_to(self, *round)),
            "an occupied round lacks quorum in its required advance window"
        );

        let fault_bound = advance.fault_bound(self.validators().len());
        rounds.into_iter().try_for_each(|round| {
            let equivocators = self
                .message_slots()
                .into_iter()
                .filter(|slot| slot.round == round && !self.equivocations(slot).is_empty())
                .map(|MessageSlot { validator, .. }| validator)
                .collect::<HashSet<_>>();
            ensure!(
                equivocators.len() <= fault_bound,
                "round {round} exceeds the equivocation bound: {equivocators:?}"
            );
            Ok(())
        })
    }
}

/// Checks communication expansion against the admission rule of the message carrier.
#[ext(name = MessageExpansionLawsExt)]
pub impl<This, H, V, C> This
where
    This: HistoryExpansion<H, Input = MessageInput<V, C>, Error = ExpansionError>,
    H: CommitteeOrder<Validator = V> + MessageCarrier<Message = C> + MessageJustification + ValidatorSet,
{
    /// Checks that expansion accepts exactly the admitted input, preserves earlier messages and
    /// justifications, places the input at its slot, and yields a lawful message DAG.
    fn check_message_expansion<Policy>(&self, history: &H, advance: &Policy, input: &MessageInput<V, C>) -> Result<()>
    where
        Policy: AdvancePolicy + QuorumThreshold,
        V: Clone + Eq + Hash + Debug,
        C: Clone + Eq + Hash + Debug,
    {
        match (self.expand(history, input), history.admit_message(advance, input)) {
            (Err(rejected), Err(admission)) => {
                ensure!(rejected == admission, "expansion rejected {rejected:?}, admission rejected {admission:?}");
                Ok(())
            }
            (Ok(later), Ok(())) => {
                ensure!(history.message_refines(&later), "expansion lost or reordered an earlier message");
                ensure!(
                    history.justification_refines(&later, &history.messages()),
                    "expansion lost an earlier justification"
                );
                ensure!(
                    later.slot_messages(&input.slot).contains(&input.candidate),
                    "expansion did not place {input:?} at its slot"
                );
                ensure!(
                    later.message_justifications(&input.candidate).into_iter().collect::<HashSet<_>>()
                        == input.justifications.iter().cloned().collect(),
                    "expansion changed the justifications of {input:?}"
                );
                later.check_lawful_message_history(advance)
            }
            (expanded, admission) => {
                bail!("expansion {:?} disagrees with admission {admission:?}", expanded.map(|_| ()))
            }
        }
    }
}

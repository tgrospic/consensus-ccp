//! Defines when the communication carrier may open a new round.
use crate::{MessageCarrier, MessageCarrierExt};
use alux_ext::ext;
use ccp_quorum::{QuorumThreshold, QuorumThresholdExt, ValidatorSet};
use std::collections::HashSet;
use std::hash::Hash;

/// Exposes how many preceding rounds must each carry quorum before a new round opens.
pub trait AdvancePolicy {
    /// Returns the number of rounds covered by the advance policy.
    fn window(&self) -> u64;
}

/// Derives round advancement from an advance window and its quorum threshold.
#[ext(name = AdvancePolicyExt)]
pub impl<This> This
where
    This: AdvancePolicy + QuorumThreshold,
{
    /// Returns how many of `total` validators may equivocate in one round.
    fn fault_bound(&self, total: usize) -> usize {
        total.saturating_sub(self.quorum_size(total))
    }

    /// Reports whether every round in the preceding window has validator quorum.
    fn advances_to<Carrier, V, C>(&self, carrier: &Carrier, round: u64) -> bool
    where
        Carrier: MessageCarrier<Validator = V, Message = C> + ValidatorSet,
        V: Clone + Eq + Hash,
    {
        if round == 0 {
            return true;
        }
        let window = self.window();
        if window == 0 || round < window {
            return false;
        }

        let validators = carrier.validators();
        (round - window..round).all(|required_round| {
            let supported = carrier
                .message_slots()
                .into_iter()
                .filter(|slot| slot.round == required_round && carrier.message_at(slot).is_some())
                .map(|slot| slot.validator)
                .filter(|validator| validators.contains(validator))
                .collect::<HashSet<_>>()
                .len();
            self.meets_quorum(supported, validators.len())
        })
    }

    /// Reports whether a proposed message carries quorum justification for every round in the window.
    fn justifications_advance<Carrier, V, C>(&self, carrier: &Carrier, round: u64, justifications: &[C]) -> bool
    where
        Carrier: MessageCarrier<Validator = V, Message = C> + ValidatorSet,
        V: Clone + Eq + Hash,
        C: Clone + Eq,
    {
        if round == 0 {
            return justifications.is_empty();
        }
        let window = self.window();
        if window == 0 || round < window {
            return false;
        }

        let validators = carrier.validators();
        (round - window..round).all(|required_round| {
            let supported = justifications
                .iter()
                .filter_map(|message| carrier.message_slot_of(message))
                .filter(|slot| slot.round == required_round && validators.contains(&slot.validator))
                .map(|slot| slot.validator)
                .collect::<HashSet<_>>()
                .len();
            self.meets_quorum(supported, validators.len())
        })
    }
}

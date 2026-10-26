//! Defines when the communication carrier may open a new round.
use crate::{MessageCarrier, MessageCarrierExt};
use alux_ext::ext;
use ccp_validator::{QuorumThreshold, ValidatorSet, ValidatorWeight, WeightedCommitteeExt};
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
    /// Reports whether every round in the preceding window carries quorum weight.
    fn advances_to<Carrier, V, W, C>(&self, carrier: &Carrier, round: u64) -> bool
    where
        Carrier: MessageCarrier<Validator = V, Message = C> + ValidatorSet + ValidatorWeight<Weight = W>,
        V: Clone + Eq + Hash,
        W: Into<u128>,
    {
        if round == 0 {
            return true;
        }
        let window = self.window();
        if window == 0 || round < window {
            return false;
        }

        (round - window..round).all(|required_round| {
            let supporters = carrier
                .message_slots()
                .into_iter()
                .filter(|slot| slot.round == required_round && carrier.message_at(slot).is_some())
                .map(|slot| slot.validator);
            carrier.weighted_quorum(self, supporters.collect::<Vec<_>>())
        })
    }

    /// Reports whether a proposed message carries quorum-weight justification for every round in the window.
    fn justifications_advance<Carrier, V, W, C>(&self, carrier: &Carrier, round: u64, justifications: &[C]) -> bool
    where
        Carrier: MessageCarrier<Validator = V, Message = C> + ValidatorSet + ValidatorWeight<Weight = W>,
        V: Clone + Eq + Hash,
        W: Into<u128>,
        C: Clone + Eq,
    {
        if round == 0 {
            return justifications.is_empty();
        }
        let window = self.window();
        if window == 0 || round < window {
            return false;
        }

        (round - window..round).all(|required_round| {
            let supporters = justifications
                .iter()
                .filter_map(|message| carrier.message_slot_of(message))
                .filter(|slot| slot.round == required_round)
                .map(|slot| slot.validator);
            carrier.weighted_quorum(self, supporters.collect::<Vec<_>>())
        })
    }
}

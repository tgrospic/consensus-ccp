/// Holds escrowed stake leaving a validator: an amount that becomes withdrawable at an epoch.
///
/// Until it matures, it still answers for the validator's offenses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Unbonding<Validator, Amount, Epoch> {
    /// Identifies the validator the stake was bonded to.
    pub validator: Validator,
    /// Holds the amount leaving.
    pub amount: Amount,
    /// Holds the epoch from which the amount may be withdrawn.
    pub matures: Epoch,
}

/// Says that a delegator asked to unbond more than it has bonded to a validator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BondShortfall<Amount> {
    /// Holds the amount bonded.
    pub bonded: Amount,
    /// Holds the amount asked for.
    pub requested: Amount,
}

/// Holds how an epoch's reward pot was settled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RewardSettlement<Amount> {
    /// Holds what was paid to validators.
    pub paid: Amount,
    /// Holds what was earned by nobody: the share withheld for weak performance, plus any remainder.
    pub unpaid: Amount,
}

/// Holds how the weight distribution changed from one epoch to another: who entered, who left, and
/// whose weight moved.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Rotation<Validator, Amount> {
    /// Holds every validator weighted in the later epoch and not the earlier, with its weight.
    pub entering: Vec<(Validator, Amount)>,
    /// Holds every validator weighted in the earlier epoch and not the later, with its weight.
    pub leaving: Vec<(Validator, Amount)>,
    /// Holds every validator weighted in both epochs whose weight differs.
    pub reweighted: Vec<Reweighting<Validator, Amount>>,
}

/// Holds one validator's weight in two epochs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Reweighting<Validator, Amount> {
    /// Identifies the validator.
    pub validator: Validator,
    /// Holds its weight in the earlier epoch.
    pub from: Amount,
    /// Holds its weight in the later epoch.
    pub to: Amount,
}

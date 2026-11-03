/// Names the validator identity and voting-weight sorts of a committee.
pub trait ValidatorSorts {
    /// The validator identity type.
    type Validator;

    /// The voting-weight type of one validator.
    type Weight;
}

/// Names the epochs over which a committee stays fixed.
pub trait EpochSorts {
    /// The epoch type, ordered in time.
    type Epoch;
}

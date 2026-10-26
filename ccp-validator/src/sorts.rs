/// Names the validator identity and voting-weight sorts of a committee.
pub trait ValidatorSorts {
    /// The validator identity type.
    type Validator;

    /// The voting-weight type of one validator.
    type Weight;
}

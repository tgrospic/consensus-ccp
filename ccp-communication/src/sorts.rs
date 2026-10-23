use ccp_quorum::ValidatorSorts;

/// Names the message sort together with its inherited validator sort.
pub trait CommunicationSorts: ValidatorSorts {
    /// Identifies a message in the communication carrier.
    type Message;
}

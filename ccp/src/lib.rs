#![doc = include_str!("../README.md")]

pub use ccp_communication as communication;
pub use ccp_confidence as confidence;
pub use ccp_core::*;
pub use ccp_leader as leader;
pub use ccp_quorum as quorum;
pub use ccp_stake as stake;
pub use ccp_token as token;
pub use ccp_validator as validator;

/// Names the independently composable protocol specifications.
pub mod protocols {
    pub use ccp_adaptive as adaptive;
    pub use ccp_cordial as cordial;
    pub use ccp_dense_leader as dense_leader;
    pub use ccp_mysticeti as mysticeti;
    pub use ccp_nakamoto as nakamoto;
}

//! Defines compact identities used by executable CCP scenarios.

use derive_more::Display;

/// Identifies one communication candidate.
#[derive(Debug, Clone, Copy, Display, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[display("{_0}")]
pub struct CandidateId(pub u64);

/// Identifies one validator.
#[derive(Debug, Clone, Copy, Display, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[display("{_0}")]
pub struct ValidatorId(pub u64);

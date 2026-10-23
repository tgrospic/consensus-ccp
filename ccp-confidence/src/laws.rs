use crate::AcceptanceThreshold;
use alux_ext::ext;
use anyhow::{Result, ensure};
use std::fmt::Debug;

/// Checks ordered confidence-threshold obligations.
#[ext(name = ConfidenceLawsExt)]
pub impl<This, K, Q> This
where
    This: AcceptanceThreshold<K, Q>,
{
    /// Checks that acceptance is upward-closed over the supplied confidence order.
    fn check_confidence_threshold(&self, _kind: K, ordered: &[Q]) -> Result<()>
    where
        Q: Ord + Debug,
    {
        ensure!(ordered.windows(2).all(|pair| pair[0] <= pair[1]), "confidence samples are not ordered");
        ensure!(
            ordered.windows(2).all(|pair| !self.accepts(&pair[0]) || self.accepts(&pair[1])),
            "acceptance is not upward-closed over confidence: {ordered:?}"
        );
        Ok(())
    }
}

//! Defines protocol-distinct confidence evidence and acceptance thresholds.

use alux_ext::ext;

/// Exposes one protocol-specific confidence interpretation for a candidate.
///
/// `K` distinguishes confidence algebras whose numerical representations must not be compared.
pub trait ConfidenceEvidence<K, C> {
    /// Protocol-specific confidence value.
    type Confidence;

    /// Returns the confidence currently accumulated for `candidate`.
    fn confidence(&self, candidate: &C) -> Option<Self::Confidence>;
}

/// Exposes the threshold that turns one confidence meaning into policy-relative acceptance.
pub trait AcceptanceThreshold<K, Q> {
    /// Reports whether `confidence` crosses this policy's acceptance boundary.
    fn accepts(&self, confidence: &Q) -> bool;
}

/// Composes protocol-specific confidence evidence with its matching acceptance threshold.
#[ext(name = ConfidenceAcceptanceExt)]
pub impl<This, K, Q> This
where
    This: AcceptanceThreshold<K, Q>,
{
    /// Reports whether the carrier has sufficient matching confidence for `candidate`.
    fn accepts_candidate<Carrier, C>(&self, carrier: &Carrier, candidate: &C, _kind: K) -> bool
    where
        Carrier: ConfidenceEvidence<K, C, Confidence = Q>,
    {
        carrier.confidence(candidate).is_some_and(|confidence| self.accepts(&confidence))
    }
}

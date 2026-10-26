# ccp-validator

Validator committee, voting weight, committee order, and weighted quorum arithmetic.

The public surface includes `ValidatorSorts`, `ValidatorSet`, `ValidatorWeight`, `CommitteeOrder`, `QuorumThreshold`, `TwoThirdsQuorum`, and their laws.

A quorum is distinct committee support whose weight strictly exceeds a threshold fraction of the total committee weight, or is the whole committee weight. The threshold 1/1 therefore means unanimity, which a genesis committee with one validator meets. Counting validators is the interpretation where every weight is one. This spec depends on no other CCP spec, so validator-management systems can implement it without the evidence layer.

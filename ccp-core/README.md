# ccp-core

Candidate and collapse sorts; lawful expansion; partial observation; restrictions; realization.

The public surface includes `CandidateSorts`, `CollapseSorts`, `CandidateSpace`, `SemanticClosure`, `HistoryExpansion`, `EvidenceExtraction`, `HorizonProjection`, `CollapseRule`, `CollapsePolicy`.

The core has no dependency on a validator system, communication representation, leader policy, or protocol spec.

Reference restrictions (`AllCandidates`, `EveryNthSlot`, `Unbounded`, `FixedDistance`) and the `u64` slot interpretation are concrete choices that protocol specifications select as parameters.

Concrete interpreters live under `crates/`. Portable obligations live with the capabilities they constrain.

A sort family can be named without implementing any operations:

```rust
use ccp_core::{CollapseSorts, CollapsePolicyExt, EvidenceExtraction, HorizonProjection, CollapseRule};

struct EvidenceWorld;
impl CollapseSorts for EvidenceWorld {
    type History = Vec<u64>;
    type Evidence = Vec<u64>;
    type Visible = Option<u64>;
    type Output = u64;
}

// One shared vocabulary links the three operations. No H/E/V/P repetition in these bounds.
fn decide<I>(interpretation: &I, history: &I::History) -> Option<I::Output>
where
    I: EvidenceExtraction + HorizonProjection + CollapseRule,
{
    interpretation.collapse_policy(history)
}
```

Candidate observations and semantic closure cannot disagree about their candidate sort:

```compile_fail
use ccp_core::{CandidateSorts, Closed, SemanticClosure};
struct World;
impl CandidateSorts for World {
    type Slot = u64;
    type Candidate = u64;
}
impl SemanticClosure for World {
    // Invalid: the candidate sort is u64, not String.
    fn closed_outcome(&self, _: &u64) -> Option<Closed<String>> { None }
}
```

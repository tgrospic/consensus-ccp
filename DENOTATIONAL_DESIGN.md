# Denotational design

## What this means

CCP's **Design by Meaning** is a Rust adaptation of **Denotational Design**, a methodology developed
and named by [Conal Elliott](http://conal.net/). Elliott's central move is to give library types and
operations precise, simple, implementation-independent meanings, and then use those meanings to
*design* the programming interface rather than merely explain an interface after implementation.

In denotational design, an implementation is not the definition of an operation. It is one
representation that must preserve the operation's meaning and relevant structure. This reverses a
common workflow:

```text
representation first                          meaning first
--------------------                          -------------
choose fields, state, and control flow        choose mathematical/domain meaning
derive an API from the machinery              derive operations compositionally
describe intended behavior afterward          choose and validate interpreters afterward
```

This is more than dependency inversion or trait-oriented programming. Traits can abstract an
implementation while leaving the intended meaning vague. Design by Meaning requires the semantic
object and its laws to be identified before representation choices are allowed to dominate the API.

## How CCP adapts it

CCP does not mechanically translate Conal Elliott's Haskell encodings into Rust. It applies the
same ordering of concerns with Rust-native tools:

| Denotational role | CCP encoding |
| --- | --- |
| Domain of meanings | Semantic carriers, candidate spaces, closures, and decisions |
| Primitive observations | Small capability traits such as `CandidateSpace` and `SemanticClosure` |
| Compositional operations | `alux_ext::ext` methods over explicit capability bounds |
| Alternative representations | Constructive machines and external interpreters of the same capabilities |
| Structure-preservation obligations | Public refinement law extensions |
| Meaning-level specification | The CCP paper plus trait signatures, extension bounds, and laws |

The Rust encoding is a design technique, not the definition of denotational design. In particular:

- a capability trait is useful only when it exposes a primitive semantic observation
- an extension method is meaningful only when its bounds and result state a compositional
  transformation
- a test is a finite executable check, not a universal proof
- the CCP paper carries claims that are broader than the current Rust law harness

When stronger assurance is required, add property tests, exhaustive finite checks, or a proof
artifact rather than describing example-based tests as a proof.

## Lineage and further reading

Start with Conal Elliott's own material:

- [Conal Elliott's website](http://conal.net/) collects his papers, talks, and writing.
- [Denotational Design: from meanings to programs](https://github.com/conal/talk-2014-bayhac-denotational-design)
  gives the short talk, abstract, slides, and video links.
- [LambdaJam Denotational Design workshop](https://github.com/conal/talk-2014-lambdajam-denotational-design)
  contains the extended workshop material and source.
- [Denotational design with type class morphisms](http://conal.net/papers/type-class-morphisms/)
  develops the structure-preserving library-design method behind the talks.
- [Haskell Interlude 62: Conal Elliott](https://haskell.foundation/podcast/62/) revisits
  denotational design, compositionality, and structure preservation in a recent interview.

The phrase "Design by Meaning" in CCP is intentionally descriptive. Credit for the underlying
methodology and the name "Denotational Design" belongs to Conal Elliott.

## Scope

These are strict design rules for CCP library authors and reviewers. They preserve the separation
between protocol meaning and concrete execution as the project grows.

## Choose the semantic carrier

The carrier is a denotation, not an imitation of network transport. It is the product of meanings
admitted at the current abstraction boundary. A carrier may include messages, causal edges,
authenticated identities, validated certificates, proof of work, stake, time, randomness, or a
conclusion established by a lower semantic layer.

No fact is absolutely internal or external. For a collapse `H × E -> O`, `E` is external relative
to `H`. Choosing `H × E` as the carrier internalizes that meaning. The revised carrier must then
state the capability and laws that make `E` meaningful.

Do not hide this choice behind a universal dependency container. Different meanings require
different capability algebras. Extension bounds expose the exact semantic row consumed by each
derived operation.

Shared shapes must preserve protocol-specific units. `ConfidenceEvidence<K, C>` and
`AcceptanceThreshold<K, Q>` compose accumulated evidence with acceptance, while `K` prevents
confirmation depth and repeated-sampling confidence from becoming accidentally comparable. Reuse
the transformation shape, not an unproved numerical identification.

## Specify meaning first

Ask what relation, observation, or transformation is being defined before asking where data is
stored or how a loop runs.

CCP's reference decision is stated as:

```text
SemanticClosure<Slot = S, Candidate = C> -> Decision<S, C>
```

The Rust witness is `ReferenceDecisionExt`. It needs semantic closure and a cloneable slot. It does
does not need concrete carrier fields, maps, sets, or a mutable consensus engine.

## Name shared sorts once

An operation-free sort trait names the carriers of a semantic domain. Capabilities inherit that
vocabulary, as in `EvidenceExtraction: CollapseSorts`, `HorizonProjection: CollapseSorts`, and
`CollapseRule: CollapseSorts`. Their composition needs only those three capability bounds; history,
evidence, visible evidence, and output are already linked by the shared sorts.

`CandidateSorts` likewise links candidate observations, closure, and candidate-based protocol
operations. `ValidatorSorts` links membership, committee order, and attributed witness selection.
`CommunicationSorts` inherits validator names and adds the message sort used by both message lookup
and justification. Naming sorts alone requires no operations or representation bounds.

Inheritance is appropriate for sharing names. Operational requirements remain explicit bounds on
derived operations. A representation-neutral policy that works with many carriers can retain type
parameters instead of choosing one sort family prematurely.

## Use tiny capabilities

A capability exposes one primitive meaning:

```rust
pub trait SemanticClosure: CandidateSorts {
    fn closed_outcome(&self, slot: &Self::Slot) -> Option<Closed<Self::Candidate>>;
}
```

The trait does not expose the map containing outcomes. This lets a reference model, simulator, DAG,
database view, or proof-oriented interpreter answer the same semantic question.

Reject representation-first APIs:

```rust
trait ConsensusContext {
    fn runtime(&self) -> &ConcreteRuntime;
    fn settings(&self) -> &Settings;
}
```

Such an API says where information lives, not what information means. It also grants every
derivation access to unrelated state.

## Derive behavior in extensions

Derived behavior belongs in an extension whose bounds state its complete semantic contract:

```rust
#[ext(name = ReferenceDecisionExt)]
impl<This, S, C> This
where
    This: SemanticClosure<Slot = S, Candidate = C>,
    S: Clone,
{
    fn reference_decision(&self, slot: &S) -> Decision<S, C> {
        // Derived only from semantic closure.
    }
}
```

Do not put derived protocol decisions in an inherent runtime implementation. That would make one interpreter
the owner of meaning and prevent other interpreters from sharing the specification.

## Separate evidence algebra from protocol vocabulary

Approval, voting, ratification, certification, and legitimacy are protocol names, not necessarily
different semantic mechanisms. Factor their shared construction before applying those names:

```text
JustificationRelation
    -> UncontestedRelation
    -> QuorumOf
    -> HorizonMatch
    -> protocol collapse continuation
```

`EvidenceRelation<W, T>` states that witness `W` supports target `T`. `UncontestedRelation` removes
support from a witness that also observes a conflicting target. `QuorumOf<Selection, Relation, Threshold>` forms a
quorum from the committee weight of distinct eligible authors selected by `Selection` whose evidence satisfies `Relation`.
The threshold is a separate choice. The existing Byzantine compositions explicitly select
`PreviousRoundWitnesses` and `TwoThirdsQuorum`. Protocol modules use type aliases and small
extensions to name compositions in their own vocabulary; they do not reimplement the algebra.

Keep horizon matching separate from its continuation. `HorizonMatch<Q, W>` returns a witness only
when the visible communication cut satisfies a pattern. `MatchCollapse<W, P>` says what order-1
output follows from that witness. Their composition has the same semantic shape as a guarded
communication rule: lawful expansion supplies evidence, a pattern matches, and only then can the
collapse continuation fire.

## Compose, do not inherit

Protocol meaning is the conjunction of capabilities in a `where` clause. Avoid supertraits that
bundle unrelated evidence:

```rust
trait MysticetiContext:
    CandidateSpace
    + ValidatorSet
    + ValidatorEvidence<CandidateSupport, Validator, Candidate>
    + ValidatorEvidence<MysticetiSkip, Validator, Slot>
    + RepairAnchorSlots
{
}
```

The bundle hides which operation needs which capability and becomes another interface that must be
implemented. Put the bounds on the operation that composes the carrier with its evidence view,
such as `MysticetiCollapseViewExt`, instead.

Value-level policy composition uses ordinary products. `Policy<R, H, L>` combines candidate,
horizon, and leader-selection policies without coupling their implementations.

## Keep interpreters constructive

Concrete carriers witness a chosen semantic world. Their representation must not let callers create
states outside that world.

- Keep carrier fields private.
- Admit change only through typed inputs interpreted by `HistoryExpansion`.
- Return a new history on success and leave the current machine history unchanged on rejection.
- Initialize protocol parameters explicitly in policy or machine composition.
- Do not expose history replacement, fact setters, authored graph edges, or authored outcomes.
- Enumerate finite futures by executing the same expansion used by the live machine.

A conclusion established by a lower layer may enter as typed input at the current boundary, but the
input name, validation capability, and laws must expose that dependency. A setter named after the
conclusion is not an interpreter.

Not every protocol uses the same product dimensions. `NakamotoPolicy` supplies confirmation depth
because proof of work discovers leaders rather than reading a scheduled `LeaderSelectionPolicy`.
`NakamotoRiskModel` separately supplies an adversarial hash-power interpretation. Prefix collapse
does not depend on that interpretation; a risk assessment composes it afterward. Omitting either
meaning is a type error only for the operation that actually consumes it.

## Put bounds at use sites

Do not constrain generic traits or carriers preemptively. A bound belongs on the operation that
uses it. This keeps primitive meanings reusable and makes extension contracts readable.

For example, `CandidateSpace` does not require slots to be ordered or candidates to be hashable.
`CordialCollapseExt` adds `S: Clone + Ord` and `C: Clone + Eq + Hash` because its derivation needs
those properties.

## State laws once

Examples show behavior for one carrier. Laws state obligations for every carrier implementing the
required capabilities.

Place reusable checks in the owning specification's `laws.rs` as extensions over capability bounds. Build integration scenarios
from typed machine inputs, and test both accepted transitions and rejection invariance. When a new
interpreter is added, run the existing laws against it rather than copying assertions.

Prefer laws about meaning:

- subset refinement
- monotonic visibility loss
- agreement with semantic closure
- idempotence, associativity, or monotonicity when an algebra supports them

Avoid tests coupled to iteration order or internal map layout unless representation is itself the
specified subject.

## Keep the product surface intentional

Root exports are part of the vocabulary users learn. Export semantic modules and types. Keep helper
topology private. A new public module should represent a stable concept, not merely a place to put
code.

## Review checklist

- Is the new meaning stated before its representation?
- Is the carrier boundary explicit about which meanings are admitted?
- Does each new trait expose one primitive capability?
- Is derived behavior an extension over minimal explicit bounds?
- Does semantic code avoid concrete fields and concrete context types?
- Are generic bounds located at the operations that require them?
- Are protocol parameters initialized by composition rather than concrete defaults?
- Can an independent interpreter implement the capabilities without using concrete carrier fields?
- Can every reachable carrier value be explained by a sequence of accepted expansion inputs?
- Is a reusable law added when the behavior implies a general obligation?
- Do scenario tests use the public product surface?
- Do public exports and documentation describe meaning rather than file topology?

If several answers are no, revise the specification before extending the implementation.

## Workspace and tagless-final terms

Keep specification crates at the repository root and concrete interpreters under `crates/`.
A generic composition belongs to the specification even when it has a Rust struct representation:
`CollapsePolicy` and `QuorumOf` carry interpretations, not concrete histories or authored outcomes.

Scenario terms follow the same discipline. Their common authoring capabilities and the
protocol-specific terms live in `ccp-scenarios`; obligations live with each protocol spec as laws.
Cross-protocol comparisons live in `ccp-comparison`; concrete interpreters live in
`crates/ccp-tests`. A communication scenario must not
require Mysticeti repair or Cordial collapse merely because another scenario uses them. Place each
bound on the operation that consumes it. The same term can be recorded for later execution or
interpreted immediately without changing its specification.

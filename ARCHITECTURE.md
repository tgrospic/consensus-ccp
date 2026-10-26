# CCP architecture

## Semantic shape

CCP treats consensus as two complementary meanings over one explicit carrier:

```text
HistoryExpansion : history × input -> Result<history, error>
HistoryCollapse  : history × query -> Option<observation>
```

Expansion constructs the lawful communication histories that can exist. Collapse observes one such
history without changing it. Candidate restriction, evidence horizon, legitimacy, leader selection,
direct decision, and repair refine this common shape.

The Byzantine protocol interpreters refine collapse into a match-and-continuation pipeline:

```text
lawful history
    -> EvidenceRelation
    -> UncontestedRelation
    -> QuorumOf
    -> horizon projection
    -> HorizonMatch<Q, W>
    -> MatchCollapse<W, P>
    -> finalized prefix
```

`HorizonMatch` is partial: it returns an evidence witness only when the visible communication cut
satisfies the requested pattern. The collapse continuation consumes that witness. This is analogous
to a `COMM` rule: expansion makes evidence resources available, matching determines whether a rule
can fire, and collapse produces the monotone order-1 observation.

The carrier is semantic rather than a packet trace. Its inputs may denote messages, authenticated
authors, validated work, certificates, time, or randomness, but each admitted meaning needs a typed
input, an expansion interpretation, and laws. Storing a conclusion in a map does not make it lawful.

## Workspace boundary

| Layer | Specification crates |
| --- | --- |
| Shared vocabulary and operations | `ccp-core`, `ccp-validator`, `ccp-quorum`, `ccp-communication`, `ccp-leader`, `ccp-confidence` |
| Stake and its asset | `ccp-stake`, `ccp-token` |
| Protocol compositions | `ccp-mysticeti`, `ccp-cordial`, `ccp-adaptive`, `ccp-dense-leader`, `ccp-nakamoto` |
| Scenario language and comparisons | `ccp-scenarios`, `ccp-comparison` |
| Specification grouping | `ccp` |
| Concrete interpretations | `crates/ccp-runtime`, `crates/ccp-tests`, `crates/ccp-stake-memory`, `crates/ccp-token-memory`, `crates/ccp-token-stake` |

All specification crates live at the repository root. Each exports its vocabulary, capabilities,
derived operations, and applicable laws. Their engineering modules remain private. The facade owns
only re-exports of specifications: the core vocabulary at its root, one path per shared spec, and
`ccp::protocols`. It does not re-export scenarios, runtime, or tests.
Interpreters depend directly on the specs they realize.

```text
ccp-core  ccp-validator  ccp-confidence
    ^          ^             ^
    ccp-quorum               |
    ^                    |
ccp-communication        |
ccp-leader               |
    ^                    |
Byzantine protocols   ccp-nakamoto
    ^
ccp-scenarios
    ^
ccp-comparison
    ^
ccp facade / concrete interpreters / examples
```

`ccp-core` and `ccp-validator` have no dependency on any other spec. `ccp-quorum` depends only on core
and validator;
`ccp-nakamoto` depends only on core and confidence; `ccp-stake` depends only on validator, and
`ccp-token` on no other spec. Protocol specs have no sibling-protocol
or runtime dependencies, and no dependency on scenario or test material. Scenarios sit above them
in `ccp-scenarios`, and cross-protocol scenarios in `ccp-comparison`.

## Sorts and operations

Sort traits declare names only. `CandidateSorts` supplies `Slot` and `Candidate`;
`CollapseSorts` supplies `History`, `Evidence`, `Visible`, and `Output`. The operations inherit
these vocabularies instead of declaring the names again. Membership, committee order, voting weight,
and witness selection inherit `ValidatorSorts`, which names `Validator` and `Weight`; lookup and justification inherit `CommunicationSorts`.

Inheritance shares vocabulary. Derived operations still list exactly the operational capabilities
they consume. A `CollapsePolicy` product connects independently selected stages by their shared
boundary sorts. Its decision extension requires `EvidenceExtraction + HorizonProjection +
CollapseRule`, with no repeated four-carrier parameter list.

## The machine boundary

`Machine<H, Expansion, Collapse>` owns exactly one reachable history. Its public operations are:

- `history`, which borrows the current carrier for semantic queries;
- `apply`, which asks `HistoryExpansion` to produce a successor and installs it only on success;
- `collapse`, which asks `HistoryCollapse` to interpret the current history.

There is deliberately no `set_history`, `add_fact`, `close`, `admit`, `add_edge`, or protocol-specific
setter. Carrier fields are private. A rejected input returns an error and leaves machine history
unchanged.

This gives the operational representation the same shape as the denotation:

```text
typed input
    -> expansion capability
    -> new reachable carrier
    -> collapse capability
    -> partial observation
```

The machine owns memory movement; it does not own protocol meaning. Expansion and collapse remain
capabilities that can be interpreted by other representations.

## Constructive communication history

`CommunicationHistory<V, C>` retains:

- an ordered genesis committee;
- canonical and equivocating messages at `(round, validator)` slots;
- direct justification links carried by each message.

The only update syntax is `MessageInput<V, C>`. `ccp-communication` specifies admission as
`MessageAdmissionExt::admit_message`, which rejects an input when:

- the author is outside the committee;
- the candidate identity already exists;
- a justification is unknown or not from an earlier round;
- the message does not justify a quorum in every round of the advance window;
- accepting it would exceed the Byzantine equivocation bound.

`CommunicationExpansion` admits the input through that rule and then inserts it. The
`MessageExpansionLawsExt` law checks that expansion accepts exactly the admitted inputs and yields a
lawful message DAG.

The reference `QuorumAdvancePolicy` is strictly `>2/3` over one previous round. The committee size is a genesis
parameter; four validators are only the default example size.

Equivocation is not overwritten or normalized away. A lawful Byzantine history retains the canonical
message and conflicting candidates in the same slot. Protocol collapse decides how that distinction
matters.

## Constructive proof-of-work history

`WorkHistory<C>` is updated only by `WorkBlockInput<C>` through `WorkExpansion`. The interpreter
rejects duplicate identities, zero work, unknown parents, and invalid genesis structure. Valid forks
remain in the carrier. `NakamotoCollapse` then composes greatest-work selection with an explicit
confirmation-depth policy.

The positive work value is an external premise at this boundary: the input documents that the proof
was verified below the CCP carrier. A signature verifier or proof verifier can be added as a separate
typed capability when that lower boundary becomes part of the executable model.

## Finite exploration

`ExpansionChoices<H, I>` enumerates candidate inputs. `FiniteExpansionExt` builds a bounded future
cone by repeatedly calling the same `HistoryExpansion::expand` used by `Machine::apply`. It cannot
author histories or edges.

`bounded_invariant_collapse` returns an observation only when every history in that bounded cone
produces the same observation. A future producing `None` prevents collapse. The bounded operation is
not presented as proof of unbounded semantic closure.

## Protocol meaning

Protocol modules remain pure capability compositions:

- Mysticeti names uncontested justification a vote and a quorum of votes a certificate, then applies
  round-robin leaders, direct skip evidence, and ordered anchor repair.
- Cordial Miners names the same common relations approval and ratification, then applies sparse wave
  leaders, ratified-predecessor repair, and approved blocklace fragments without Mysticeti's
  pipeline or negative certificate.
- Adaptive composes >2/3 legitimacy with adaptive horizon evidence and certified predecessors.
- Dense leader selects every validator slot over the same observer-relative legitimacy relation.
- Nakamoto composes block-tree structure, cumulative work, confirmation depth, and separate reversal
  risk.

Adaptive does not yet have an end-to-end message interpreter. Its missing derivations are not
replaced by setters that assert `super_ratified`, `final`, `certified`, or `closed`.

## Adding meaning

1. State the primitive meaning as a small capability or typed input.
2. Define the lawful expansion that admits it, including rejection conditions.
3. Keep the carrier field private and expose only semantic observations.
4. Derive protocol behavior in `alux_ext::ext` methods over minimal bounds.
5. Add reusable laws and a scenario built entirely through `Machine::apply`.
6. If finite analysis is needed, enumerate inputs and execute the same expansion.

If a scenario needs a direct setter for the expected conclusion, the interpreter boundary is not
finished.

## Composing interpretations

`CollapsePolicy<L, R, C>` is a product of independent interpretations of `EvidenceExtraction`,
`HorizonProjection`, and `CollapseRule`. Its components need only implement their own capability;
`CollapsePolicyExt` derives the decision map from the three operations. The realization law checks
both the individual stages and their induced decision map. The conformance runner exercises this
law on a positive Mysticeti direct-commit composition, using list and set representations of
certificate authors over reachable histories. This checks that composition's representation
independence; it is not a proof of the entire protocol.

`QuorumOf<Selection, Relation, Threshold>` composes attributed witness selection, support, and quorum
arithmetic. `WitnessQuorumExt` weighs distinct eligible authors against the total committee weight
from `ccp-validator`; counting is the unit-weight case. `PreviousRoundWitnesses` selects
immediately preceding direct justifications; the Byzantine protocol compositions explicitly pair it
with `TwoThirdsQuorum`. Neither temporal adjacency nor the threshold is built into generic quorum
formation. Selecting a different threshold defines a different policy whose safety obligations must
be established separately.

`ccp-scenarios` contains the common authoring capabilities and the scenario terms of each protocol.
Fixed-expectation checks live with the runners in `crates/ccp-tests`. Test code imports the
scenario crates directly. Each operation declares only the capabilities it consumes. The recorded runner and the immediate
communication runner in `crates/ccp-tests` interpret the same communication scenario terms. The
immediate runner implements no protocol collapse or repair capabilities. These are two scenario
interpretations using the same constructive carrier, not two independent consensus implementations.

## Remaining semantic boundaries

Paper Definition 2.6 requires at least one defined observation and agreement among all defined
observations in the admissible future cone. `bounded_invariant_collapse` instead requires every
explored history, including the initial history, to have the same defined observation. Its stronger
finite condition remains a separate analysis operation. `SemanticClosure` still names an oracle
capability; this reorganization does not supply an unbounded closure interpreter.

For a growing output, preservation of an emitted prefix must be distinguished from equality of a
fixed terminal observation. The existing prefix-extension laws express growth; the paper's equality
criterion needs that observation boundary made explicit before the two can be identified.

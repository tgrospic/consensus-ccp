# CCP repository guide

This file is the compact engineering context for humans and coding agents working on CCP.
Apply it to the whole repository.

## Product

CCP is a Rust specification framework for understanding consensus as expansion and collapse over an
explicit semantic carrier. It compares protocols by their lawful communication expansions, the
candidate space they retain, the evidence horizon they observe, their collapse rule, and their repair
rule.

This repository is a specification and comparison library. Do not turn it into a networking,
storage, validator-management, or node-runtime project without an explicit scope decision.

## Workspace

| Spec / interpreter | Responsibility |
| --- | --- |
| `ccp/` | Grouping of the specifications; re-exports specs only |
| `ccp-core/` | Candidate and collapse sorts, expansion, restrictions, realization |
| `ccp-quorum/`, `ccp-communication/` | Validator and message sorts, message-carrier admission, evidence compositions |
| `ccp-leader/`, `ccp-confidence/` | Scheduling and protocol-distinct confidence |
| `ccp-mysticeti/`, `ccp-cordial/`, `ccp-adaptive/`, `ccp-dense-leader/`, `ccp-nakamoto/` | Protocol specifications and their laws |
| `ccp-scenarios/` | Scenario-authoring language and protocol scenarios |
| `ccp-comparison/` | Cross-protocol scenario compositions |
| `crates/ccp-runtime/` | Constructive carrier and machine interpretations |
| `crates/ccp-tests/` | Concrete conformance runners |
| `crates/ccp-examples/` | Executable protocol comparisons |

Specification crates live at the repository root. Dependencies point toward shared specs; no spec
may depend on the facade or a concrete interpreter. Protocol specs do not depend on sibling
protocols. Cross-protocol comparisons belong in `ccp-comparison`.

## Authority

Read authority in this order:

1. The paper defines the conceptual vocabulary and claims.
2. Capability trait signatures define primitive executable meaning.
3. Extension bounds and methods define derived meaning.
4. Public law extensions define reusable obligations.
5. Reference interpreters and integration tests witness the meaning.
6. Prose summarizes the artifacts above.

If the paper and executable specification disagree, do not silently choose one. State the mismatch
and update them together or record the unresolved boundary.

## Read order

For design work, read from meaning to machinery:

1. `README.md`
2. `ARCHITECTURE.md`
3. `ccp-core/src/sorts.rs` and `ccp-core/src/candidate.rs`
4. `ccp-core/src/expansion.rs` or `ccp-core/src/realization.rs` for expansion, closure, and realization work
5. `ccp-core/src/restriction.rs`
6. the relevant protocol module
7. the owning specification's `laws.rs`
8. the relevant concrete interpreter and integration test

Use `DENOTATIONAL_DESIGN.md` for design and review rules.

## Commands

The CI-equivalent local commands are:

```sh
cargo fmt --all -- --check
cargo build --workspace --all-features --tests --bins --benches --examples
cargo clippy --workspace --all-features --tests --bins --benches --examples -- -D warnings
cargo doc --workspace --no-deps --document-private-items
cargo nextest run --workspace --all-features --no-fail-fast
cargo test --workspace --all-features --doc
cargo publish --workspace --dry-run
```

The `Justfile` provides the same workflows. Run the narrowest relevant test during development and
the full checks before finishing a change.

## Design by meaning

This is CCP's Rust adaptation of Conal Elliott's Denotational Design. Read the lineage and limits in
`DENOTATIONAL_DESIGN.md`; do not reduce the method to "use traits and extension methods."

- Ask what value, relation, or transformation is being specified before choosing a representation.
- Treat the carrier as a chosen semantic world, not a fixed packet or storage representation.
- State which meanings are admitted to the carrier and which are supplied separately to a collapse.
- Treat externality as relative to that boundary: enriching the carrier can internalize a meaning
  only when its capability and laws are made explicit.
- Define primitive meaning as small capability traits.
- Put derived behavior in `alux_ext::ext` implementations over explicit `where` clauses.
- Treat an extension's bounds as its semantic dependency declaration.
- Declare shared semantic type names once in operation-free sort traits. Capabilities may inherit
  those sort traits; sort traits may inherit other sort vocabularies. Compose operational
  capabilities with explicit bounds rather than bundling unrelated operations in supertraits.
- Use direct capability bounds when one interpreter answers them. Use an explicit policy parameter
  or product for separately selected behavior. Use `HasX` projections when an environment carries
  independent interpreters. Use delegation only for truthful substitution.
- Keep protocol modules independent from concrete interpreters.
- Keep carrier fields private. A carrier may change only by interpreting a typed input through
  `HistoryExpansion`.
- Keep concrete machines thin. They own one reachable history, delegate transitions to expansion,
  and delegate observations to collapse.
- Never expose history replacement, semantic-fact setters, authored expansion edges, or authored
  collapse outcomes.
- Enumerate finite future cones by executing the same expansion capability as the live machine.
- Keep concrete defaults representation-neutral. Protocol-specific initialization belongs in a
  policy composition, fixture, or explicit constructor.
- Add a capability only when a new primitive meaning is required.
- Add an abstraction only when it removes semantic duplication or expresses real composition.

Target shape:

```text
tiny capability traits
    -> derived extension methods
    -> neutral interpreter implementations
    -> shared laws
    -> protocol compositions and scenarios
```

Reject these shapes:

- a manager or context trait exposing the whole implementation
- semantic logic that reads concrete carrier fields directly
- a public fact bag that lets callers manufacture protocol conclusions
- a finite explorer with authored histories, edges, admissibility flags, or observations
- protocol parameters hidden in `Default`
- one large trait containing unrelated protocol evidence
- one universal enum or context that erases distinct external meanings
- a second operational implementation presented as a specification

## Rust conventions

- Rust edition and minimum version are inherited from the root workspace manifest.
- Declare every dependency in root `[workspace.dependencies]`; member crates use
  `dependency.workspace = true`.
- Use major versions for stable dependencies and minor versions for `0.x` dependencies.
- Prefer standard derives. Use a hand-written implementation only when it carries necessary
  representation behavior, not to smuggle semantic policy into a carrier.
- Avoid bounds on trait and struct definitions unless the definition is meaningless without them.
  Put bounds at use sites.
- Use `alux_ext::ext` for derived operations.
- Import generated extension traits and call their methods directly. Do not use extension traits as
  semantic bounds or fully qualified extension calls unless genuine ambiguity requires it.
- Name associated types as type parameters when this makes bounds clearer. Use
  `<This as Trait>::Assoc` only to resolve genuine ambiguity.
- Group imports and re-exports by visibility, and keep same-visibility imports together.
- Prefer iterator combinators such as `map`, `filter_map`, `flat_map`, and `try_for_each` when they
  make the dataflow clearer than an explicit loop.
- Use `derive_more` 2.x for `Display` and related wrapper derives when a plain standard derive is
  not available. Enable only the required derive features; do not enable `full` without a
  demonstrated need.
- Use `derive-new` for simple `new()` constructors when that keeps the type definition thin.
- Root modules define the product surface. Re-export intentionally; keep engineering submodules
  private unless their module path is part of the public vocabulary.
- Order declarations by visibility: `pub`, `pub(crate)`, then private. Keep imports at module scope.
- Crate-local macros use `macro_rules!` and `pub(crate) use`. Use `#[macro_export]` only for a real
  external API.
- Unsafe code is forbidden by workspace lint policy.
- Public items require concise documentation describing meaning.
- Documentation uses US English. Doc comments begin with a third-person singular verb where
  practical.

## Tests and laws

- Test meaning through public capabilities and extensions, not private representation.
- Put reusable obligations in the owning spec's `laws.rs` so every interpreter can run them.
- Keep scenarios out of specs: scenario terms live in `ccp-scenarios` or `ccp-comparison`, and
  fixed-expectation checks live in `crates/ccp-tests`.
- Build integration scenarios from typed inputs executed by the public machine.
- Assert that rejected inputs leave machine history unchanged.
- Add at least one law or focused assertion with every new semantic operation.
- Prefer equality and refinement checks over snapshots of execution steps.
- A protocol-specific result may restrict or defer the reference result, but a closed result must
  not contradict semantic closure.

Relevant law families include:

- candidate restriction only removes candidates
- evidence-horizon restriction only hides evidence
- closed protocol decisions refine unrestricted semantic closure

## Change workflow

1. State the semantic change in one sentence.
2. Identify the smallest primitive capabilities it needs.
3. Express derived behavior as an extension over those capabilities.
4. Interpret new inputs in `ccp_runtime` only when an executable witness is useful; do not add a
   setter for the resulting fact.
5. Add or extend reusable laws.
6. Add a concrete scenario that demonstrates the composition.
7. Update architecture and paper-facing prose when the conceptual surface changes.
8. Run formatting, tests, Clippy, rustdoc, and package verification.

Use Conventional Commits 1.0.0 for commit messages. Format subjects as
`<type>(<scope>): <imperative summary>`.

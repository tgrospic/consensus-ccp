# CCP conformance runners

Interprets the portable laws from the `ccp-*` specs and terms built over
[`ccp-scenarios`](../../ccp-scenarios). Cross-protocol terms live in `ccp-comparison`. Fixed-expectation
checks used by the runners live here in `expectations.rs`.
The recorded runner and immediate communication runner execute the same scenario specifications
through the public constructive machine. The immediate runner needs only communication capabilities.

Composition checks vary witness selection and threshold independently, and check the realization
law for a positive direct-commit composition using list and set representations of evidence.

The law runner also checks the Adaptive anchor laws from every observer of each Mysticeti and scheduled-leader scenario, and that every evidence view weighs a slot by its epoch's committee. `handover.rs` witnesses safety across an epoch boundary with a vote protocol whose committee changes by half: certificates weighed by their own epoch stay safe, weighing by another epoch's committee breaks agreement, and gating the next epoch on the closed prefix keeps handover continuity.

Run `cargo nextest run -p consensus-ccp-tests --all-features`.

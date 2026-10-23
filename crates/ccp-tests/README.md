# CCP conformance runners

Interprets the portable laws from the `ccp-*` specs and terms built over
[`ccp-scenarios`](../../ccp-scenarios). Cross-protocol terms live in `ccp-comparison`. Fixed-expectation
checks used by the runners live here in `expectations.rs`.
The recorded runner and immediate communication runner execute the same scenario specifications
through the public constructive machine. The immediate runner needs only communication capabilities.

Composition checks vary witness selection and threshold independently, and check the realization
law for a positive direct-commit composition using list and set representations of evidence.

Run `cargo nextest run -p consensus-ccp-tests --all-features`.

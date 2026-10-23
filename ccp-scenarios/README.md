# ccp-scenarios

Representation-neutral scenario-authoring capabilities, common communication scenarios, and the scenario terms of each protocol.

The public surface includes `Scenario`, `ScenarioHistory`, `CommunicationHistoryScenario`, `ObservedDecisions`.

The authoring language is representation-neutral and has no dependency on concrete interpreters. Protocol scenario terms depend on the protocol specs whose policies they select, so this crate sits above them; protocol specs never depend on it. Recorded and immediate interpreters live under `crates/ccp-tests`.

Concrete interpreters live under `crates/`. Portable obligations live with the capabilities they constrain.

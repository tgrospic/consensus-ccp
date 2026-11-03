# ccp-stake-memory

Interprets [`ccp-stake`](https://docs.rs/ccp-stake) over ordered maps. `MemoryStake` keeps wallets as its own custody and holdings, and takes every policy the specification leaves open as a parameter: the weight source, snapshot lag, unbonding delay, quorum fraction, emission per epoch, evidence window, and churn quotient. Offenses forfeit their value in basis points; performance is blocks produced out of blocks expected, and scales each validator's share of the pot by weight. It is the reference interpretation the specification's laws run against.

`Policy::default()` chooses values that satisfy every law, so a test changes one value to show which law it breaks:

| Policy | Default | Law it serves |
| --- | --- | --- |
| `source` | escrow | the weight source |
| `lag` | 1 epoch | `check_snapshot_precedes` |
| `unbonding_delay` | 2 epochs | `check_unbonding_answers`: at least the lag plus the evidence window, less one |
| `evidence_window` | 2 epochs | `check_unbonding_answers` |
| `quorum` | 2/3 | the `ccp-validator` quorum laws |
| `churn_quotient` | 8: the limit is `(total − 1) / 8` | `check_churn_limit`: at 2/3 the quotient must be at least 5 for quorums to intersect |
| `emission` | 1,000 per epoch | `check_reward` |

# ccp-token-stake

Composes [`ccp-token`](../../ccp-token) and [`ccp-stake`](../../ccp-stake) into chains whose stake is their native token. Neither specification names the other; a chain is the only place they meet.

`Chain<Variant>` keeps the carriers. Each variant answers `SnapshotCloseAlg`, how its closing snapshot admits stake. The token ledger, epochs, snapshots, exclusion, and observed performance are written once for every variant; bonds, unbonding, and custody are written once for every `Escrowed` variant. Each variant marker answers only the primitives where it differs:

| | `Cosmos` | `Ethereum` | `Cardano` |
| --- | --- | --- | --- |
| Weight source | escrow | escrow, capped at the maximum effective balance, admitted through a churn queue of an eighth of the weight per epoch | delegation: balances point at a pool, nothing moves |
| Snapshot lag | one epoch | one epoch | two epochs |
| Emission | one percent of supply, minted | four times the square root of the total weight, minted | three tenths of a percent of the reserve, transferred |
| Reward share | by weight | by weight, scaled by duties performed out of assigned | by weight, scaled by blocks produced out of expected |
| Misbehavior | forfeit (burned) and jailing | forfeit for equivocation and for inactivity (quadratic leak), and ejection | withheld rewards only |

## Parameters and their mainnet counterparts

The chains are small models: an epoch is a step of the model, and amounts are small integers so tests stay readable. Each table sets the value a chain here chooses beside the rule the mainnet uses. A value here keeps the mainnet's shape (what depends on what) and scales the numbers down.

### `Cosmos`, against the Cosmos Hub

| Parameter | Here | Cosmos Hub |
| --- | --- | --- |
| Epoch | one model step | no epochs: validator updates from a block take effect two blocks later |
| Snapshot lag | one epoch | two blocks |
| Churn limit | none | none; the active set is capped by `max_validators` |
| Unbonding delay | three epochs | 21 days |
| Evidence window | three epochs, as long as unbonding | consensus parameters `evidence.max_age_duration` and `max_age_num_blocks`, kept within the unbonding period |
| Quorum | more than two thirds of weight | more than two thirds of voting power |
| Equivocation forfeit | basis points chosen by the test | 5%, and the validator is tombstoned (excluded for good) |
| Downtime | jailing by the test | 0.01% forfeit and 10 minutes jailed after signing under 5% of the last 10,000 blocks |
| Emission | 1% of supply per epoch | yearly inflation between a governance minimum and maximum, adjusted toward a bonded-ratio goal |

### `Ethereum`, against Ethereum mainnet (Electra)

| Parameter | Here | Ethereum |
| --- | --- | --- |
| Epoch | one model step | 32 slots of 12 seconds (6.4 minutes) |
| Snapshot lag | one epoch | activation and exit take effect `1 + MAX_SEED_LOOKAHEAD` = 5 epochs after they are processed |
| Churn limit | an eighth of the admitted weight, less one, per epoch | balance churn `max(128 ETH, total / 65,536)` per epoch, activation and exit capped at 256 ETH |
| Effective balance cap | 2,048 | 2,048 ETH (`MAX_EFFECTIVE_BALANCE_ELECTRA`); 32 ETH for validators without compounding credentials |
| Unbonding delay | four epochs | after exit, `MIN_VALIDATOR_WITHDRAWABILITY_DELAY` = 256 epochs (about 27 hours); a slashed validator waits `EPOCHS_PER_SLASHINGS_VECTOR` = 8,192 epochs (about 36 days) |
| Evidence window | four epochs, as long as unbonding | a validator is slashable until it becomes withdrawable, so the window is the withdrawability delay itself |
| Quorum | more than two thirds of weight | Casper FFG: two thirds of the active balance |
| Equivocation forfeit | a thirty-second of the stake | an initial `1 / 4,096` of the effective balance (a thirty-second from Bellatrix to Deneb), plus the correlation penalty halfway through the slashing window |
| Inactivity | quadratic in the epochs missed, over 4,096 | a per-validator inactivity score, penalty `score × balance / 2^24`, quadratic over a leak |
| Ejection | exclusion by the test | forced exit when the effective balance falls to 16 ETH |
| Emission | four times the square root of the total weight | base reward per validator proportional to `1 / √total`, so total issuance grows with `√total` |

### `Cardano`, against Cardano mainnet

| Parameter | Here | Cardano |
| --- | --- | --- |
| Epoch | one model step | 5 days |
| Snapshot lag | two epochs | the stake snapshot taken at the end of epoch `e` selects leaders in epoch `e + 2` |
| Churn limit | none | none |
| Unbonding delay and evidence window | none: stake is never escrowed or forfeited | none: stake stays liquid in wallets, and nothing is slashed |
| Quorum | none | none: Ouroboros Praos follows the longest chain |
| Emission | 0.3% of the reserve per epoch | monetary expansion `ρ` = 0.3% of the reserve per epoch, of which `τ` = 20% goes to the treasury |
| Reward share | by weight, scaled by blocks produced out of expected | by stake and the pool's apparent performance, capped by saturation (`k` = 500) and adjusted by pledge (`a0` = 0.3) |

TODO: grow an `Ethereum` forfeit with how much other stake was forfeited in the same window (the correlation penalty), once the stake specification observes recent forfeits.

The tests state what each composition keeps: on every variant, rotation is settled for each epoch and is exactly the difference of epoch weights; escrow keeps the supply and the pool holds exactly what is escrowed; forfeiting burns exactly what it takes; on escrow variants, unbonding outlasts the evidence window of every epoch the stake weighed; Ethereum's churn queue spreads a large entry over epochs and keeps every handover within the limit; inflation raises the supply by exactly what it pays; Cardano rewards move tokens out of the reserve without changing the supply, and withholding never touches principal.

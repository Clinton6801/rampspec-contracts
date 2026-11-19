# Contract Upgrade Policy

The evidence registry accepts an upgrade only when it is initialized, unpaused, authorized by the current administrator, and given a nonzero uploaded WASM hash different from the recorded hash.

Soroban contracts cannot inspect a candidate WASM's generated ABI or prove storage-schema compatibility on-chain. The release pipeline therefore owns compatibility checks: generated spec diff, storage version review, testnet rehearsal, security approval, and published checksums must all pass before the administrator signs `upgrade`.

The first upgrade uses a zero hash as the `old_wasm_hash` event value because Soroban does not expose the executing WASM hash to contract code. Deployment manifests remain the authoritative record of the initial code hash. Every successful upgrade stores the new hash for subsequent transition events.

Administrator transfer remains two-step: the current administrator proposes a successor and the proposed address authorizes acceptance. Before acceptance, the current administrator may call `cancel_admin_transfer`. Cancellation clears only the pending administrator, preserves the current administrator, extends instance TTL, and emits `admin_proposal_cancelled`. Missing proposals and unauthorized attempts leave state, TTL, and events unchanged.

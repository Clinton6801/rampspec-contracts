# Storage TTL Policy

RampSpec assumes five-second ledgers for human-readable estimates. Ledger counts, not wall-clock timestamps, are authoritative.

| State | Extend below | Extend to | Approximate target |
| --- | ---: | ---: | ---: |
| Contract instance and code | 103,680 ledgers | 2,073,600 ledgers | 120 days |
| Attestors, evidence, and indexes | 518,400 ledgers | 3,110,400 ledgers | 180 days |

Every relevant write extends its state. Successful `schema_version`, `get_attestor`, `get_evidence`, and `is_active` reads extend only the contract instance TTL; missing attestor or evidence reads do not extend TTL or create state. This keeps frequently verified registry configuration and code live while leaving persistent-entry retention under explicit operator control. Callers pay the additional instance-rent bump when the threshold is reached.

The operator maintenance service must inspect and refresh live entries at least every 30 days using `maintain_attestor` and `maintain_evidence`; calls are administrator-authorized and fail for unknown or archived entries. `maintain_evidence` refreshes an existing record regardless of status, repairs a missing active uniqueness index only for an active record, rejects a conflicting index before TTL changes, and never recreates an active index for revoked or superseded records.

Use administrator-authorized `maintain_supersession_chain` with the first superseded record to refresh every evidence record and forward link through the terminal successor to at least the 180-day target. The operation validates the complete forward chain, including each successor record and its backlink, before changing any TTL. It rejects missing records, incomplete links, and chains longer than 23 links without TTL changes or events. Restore archived records and links before maintenance.

Maintenance cannot restore an archived entry from inside the contract. Operators must submit the network restore-footprint transaction first, then call maintenance. Contract events and signed release/deployment manifests are the recovery source if an indexer cache is lost. Alert at 45 days remaining, escalate at 30 days, and freeze new publication if the instance drops below 14 days without a successful extension.

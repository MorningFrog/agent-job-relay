# ADR 0020: Scoped user consent before goal-bound admission

## Decision

Treat missing pause permission as structured pending input, not a runtime error. Store one durable request pinned to actual registration/endpoint, original goal/controller, workspace, label and a 1–16-job limit. Pending preparation is reused without renewal; approval is adopted only through the explicit request ID. Original admission consumes a slot in the existing SQLite transaction, while accepted-key retries retain IDs without consuming budget. Schema v11 adds consent/audits and an optional wait reference, independently of execution and native pause acknowledgement.

Use negotiated MCP forms for actual host input: legacy elicitation or modern input-required with opaque server-held token and matching request-keyed response. Require accepted action and explicit boolean approval, without defaults. If form support is absent, claim one Agent question and record only an actual host answer through a revision-fenced assertion. Repeated calls never create another question. Plugin text, silence and preselection are not consent; native pending tokens cannot be overridden with an Agent assertion. Cooperative Agent/terminal assertions remain distinguishable from native form answers, without claiming host attestation.

Pending requests last ten minutes, claimed questions five minutes and approved admission 24 hours. Expiry is an admission boundary, not a workload timeout. Decline/expiry never automatically reopen the request. Revocation, observable goal/controller/caller changes and backup restore fence future control/notification without stopping or replaying admitted workloads. Retain bounded original Job/Run/wait IDs for uncertain-admission recovery. Histories are bounded by the existing style of durable record quotas, not destructive pruning.

## Consequences

A natural user reply covers later rounds of the same bounded request/goal without fixed wording or repeated confirmation. Requested delays belong inside each round’s workload; separately requested calls remain separate. Native pause acknowledgement and uncertain-mutation non-replay remain mandatory under [ADR 0017](0017-opted-in-goal-waiting.md). Consent cannot transfer to another goal/chat/workspace. Host generation/CAS is still unavailable. Real native-form UI and complete multi-round host acceptance are distinct from protocol fixtures and the user-verified approval→pause result.

## Verification

Reuse storage/fake-host infrastructure for native accept/cancel, one-question fallback, token/revision/caller/workspace fences, expiry/deletion/revocation/restore, three separate actual admissions and idempotent budget preservation. Retained-worker coverage includes both protocol families and a human wait longer than the active transport budget. No fixture may create or mutate a real user goal.

# ADR 0006: bounded outage recovery and delivery-time lease renewal

Accepted, 2026-10-26, source version 0.1.3. No personal installation is changed.

Five rapidly exhausted attempts did not span normal ChatGPT Desktop downtime. Temporary failures now retry with exponential backoff/jitter capped at five minutes, bounded by the original 24-hour event TTL. Identity failures wait for explicit recovery; permanent capability/catalog errors stop. Delivery queries expose classification and scheduled time; accepted_by_host still provides no Agent-execution evidence.

`session recover` refreshes/probes current-terminal identity and previews targets; `--apply` or `job_recover_session` explicitly recovers only identical logical chat/caller/host identities. The transaction fences registration changes and live claims, creates an immutable target when needed, and transfers only unresolved unexpired deliveries with original IDs, attempts and expiry. Accepted events are excluded. Existing canonical deliveries are coalesced with a superseded-by link. Target transfer and audit commit together. No command, historical chat scan, global latest-session lookup or implicit state-directory switch is involved.

Automatic bindings opt into `renew_on_delivery`; older/manual registrations default false. A result claim whose session is expired or within one hour of expiry first probes the original endpoint owner/fingerprint/capability. A fenced transaction rechecks registration, expiry, connected state and active claim before extending the configured lease and recording an audit. There is no periodic host polling. Changed endpoints or explicit invalidation cannot be silently renewed. The existing three-second attempt deadline also bounds the optional renewal.

Schema v5 adds failure_kind; v6 adds target_recoveries. These migrations preserve records, but older executables refuse initialization of newer schemas. Doctor capabilities let new automatic binders refuse old daemons before registration/job mutation. Updates require an explicit same-profile idle stop; no live daemon or personal install/cache is automatically replaced.

ChatGPT Desktop's private API still cannot attest caller ownership independently or prove resume/deduplication. Tests use isolated installed CLI/MCP entry points and fake hosts, plus simulated historical expiry. They do not qualify real ChatGPT Desktop restart, multi-day execution or new live continuation. Native Windows/macOS remain pending.

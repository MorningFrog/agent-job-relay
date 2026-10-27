# ADR 0012: Automatic Windows daemon bootstrap

## Decision

The user requested automatic startup with as little manual preparation as possible. Replace ADR 0011's direct-only daemon startup with an on-demand, same-user local CIM bootstrap when native creation fails with access denied inside a Job Object. Keep direct native runner launch and Job Object containment unchanged. No startup entry, scheduled task, credential, service installation or persistent helper is needed.

Windows documents that `Win32_Process.Create` does not inherit the caller's Job Object. Its provider can impose its own job limits; the helper must still successfully launch an independent native daemon. Invoke the built-in Windows PowerShell/CIM interface without profiles, remote targets or elevation. Only the absolute current executable, fixed helper mode, explicit state path and private rendezvous identity enter that command. Job commands never enter CIM. Hidden new-console startup is used because the provider rejects the attempted no-window creation flag combination.

Before transferring anything, validate the helper's returned PID against a pinned process handle, creation time, current-user SID, integrity level, privilege inventory, executable image and connected pipe client. The helper independently validates the server instance/image/token and pipe DACL. Reject restricted tokens, AppContainers and integrity below medium; the broker must not turn a sandbox token into a broader process under the same account.

Pass the version, exact UTF-16 environment and cwd over the random exclusive current-user/System-only pipe with a bounded frame. Never persist environment values or put them in command lines/reports. Remove inherited chat identities; restore the caller environment when the helper natively launches the same profile's daemon. A missing/wrong caller, version conflict, disconnect, excess frame, policy denial or timeout fails without job submission. A helper that never receives a valid transfer expires. Existing startup/daemon locks serialize competing requests; no failed launch is interpreted as permission to switch profiles or replay work.

The daemon reports its launch method through doctor. Default `AJR_WINDOWS_STARTUP=auto` tries direct startup first; `direct` disables CIM. Startup has a 20-second outer deadline, a bounded CIM command and helper handshake. Existing IPC, delivery and workload deadlines remain unchanged. An administrative policy can deny CIM; the fallback is one same-profile `daemon start` command in an outside terminal, not an automatic privilege or account change.

## Verification

Native regressions cover cold launch inside a non-breakaway job, Unicode/quoted paths and environment, removal of chat identities, capture EOF, destruction of the host's cleanup job, daemon-crash restart without replay, direct-only refusal, incorrect parent instances/images and restricted/low-integrity token refusal. Actual installed Codex model/MCP acceptance must begin with no prestarted daemon. Ordinary interactive-user and real Windows Desktop acceptance remain separate from guest-agent SYSTEM tests.

## Sources

- [Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)
- [Win32_Process.Create](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/create-method-in-class-win32-process)
- [Win32_ProcessStartup](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-processstartup)

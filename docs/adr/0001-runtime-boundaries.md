# ADR 0001: durable intent, independent runner, conservative recovery

Accepted. Prefer one package with small modules. Runners write SQLite directly on bounded blocking threads: finalization does not depend on daemon availability. OS file locks + transactional CAS fence launch and recovery. Uncertain launches are classified orphaned instead of replayed. This trades automatic retry for avoiding duplicate side effects. One run per job in v0.1; reruns are new jobs.

Linux-native IPC and process groups first. A target must not enable runtime startup without its native identity and IPC ownership implementation. Windows uses the native pipe/Job Object adapter in ADR 0011; experimental, untested macOS ARM64 source is defined by ADR 0024. This is a scoped milestone, not a claim that Linux process semantics generalize to Windows.

Implementation detail: Linux runner is a child subreaper. Ordinary errors use an owned group cleanup guard; SIGKILL cannot run it. Windows now uses its separate native adapter; the untested macOS ARM64 process-group adapter has the weaker containment boundary in ADR 0024. IPC has version 1; schema migrations currently advance through v4. Run state/timestamps are persisted independently from the job intent (one attempt per job for this milestone).

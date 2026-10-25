# ADR 0001: durable intent, independent runner, conservative recovery

Accepted. Prefer one package with small modules. Runners write SQLite directly
on bounded blocking threads: finalization does not depend on daemon availability.
OS file locks + transactional CAS fence launch and recovery. Uncertain launches
are classified orphaned instead of replayed. This trades automatic retry for
avoiding duplicate side effects. One run per job in v0.1; reruns are new jobs.

Linux-native IPC and process groups first. macOS/Windows builds may expose help
and setup, but runtime support must remain explicitly unavailable without native
identity, pipe ACL and Job Object implementations. This is a scoped milestone,
not a claim that Linux process semantics generalize to Windows.

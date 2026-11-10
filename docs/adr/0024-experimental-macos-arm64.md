# ADR 0024: Experimental macOS ARM64 through cross-compilation

## Decision

Keep the single Rust executable and existing IPC/storage contracts. Add a Darwin-specific adapter behind the shared Unix implementation and allow native runtime startup only for Apple Silicon. Build from Linux through a user-supplied osxcross toolchain and macOS SDK; never spoof Linux process identity or execute target binaries on the build host. The current work explicitly excludes tests and native acceptance.

Process identity combines `kern.bootsessionuuid`, PID and `proc_pidinfo` start time. Tokio's Darwin peer credentials preserve same-user socket checks. Use `renamex_np(RENAME_EXCL)` for new backup/profile publication and directory `fsync`; keep regular-file synchronization and private-file protections. Compare the loaded Mach-O UUID with the on-disk executable before launching daemon/runners, and refuse replacement instead of using an unrelated new image. Active runners must finish before Mac package replacement.

Darwin process groups do not provide Linux subreaper adoption or `PDEATHSIG`. The experimental adapter explicitly exposes that escaped descendants and abrupt runner loss have weaker cleanup guarantees; it must not advertise Linux-equivalent process-tree containment. Normal cleanup verifies group exit independently of child reaping. Stronger containment is outside the current open-loop implementation.

## Delivery boundary

Cross-built native descriptors and generated npm target metadata remain explicitly untested. Mac bytes can enter only development packages; formal candidate/sealing/publication requirements remain Linux/Windows and unchanged. Published artifacts and source-only Intel macOS portable checks retain their original meanings. This supersedes the blanket macOS-unimplemented statements in ADRs 0001/0011 for current development source, without extending their Linux/Windows acceptance evidence.

Actual target compilation uses a user-supplied SDK/toolchain; the selected SDK/linker combination and build result are recorded in implementation status. SDK 27.x with the LLVM flavor needs LLD >=23.1.3 for its arm64e.x1 TAPI targets; SDK 26.5 works with LLD 21. Cross-compilation, static checks and reviewed source cannot establish Mac execution, filesystem durability, signature validity, installation or host continuation. See [the development guide](../macos-arm64.md) and [implementation status](../IMPLEMENTATION_STATUS.md).

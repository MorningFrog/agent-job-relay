# ADR 0008: recover interrupted plugin directory replacements

Accepted, 2026-10-26, source 0.1.5.

## Problem

Installation/cache updates move the original directory to a backup before installing the staged tree. Process exit between renames can leave the stable path missing. In-process error rollback cannot run after termination, and blindly reinstalling could lose ownership or select a different state profile.

## Decision

Stage and sync the complete new tree, then atomically publish a private sibling journal containing the exact destination, operation UUID, installation/state identities and original/new file hashes. Sync the parent before destructive renames and after each rename. Locks remain independent of directory paths. Pending journals block normal lifecycle writes.

`plugin repair --path` inspects only that explicitly selected operation, defaults to preview, and finishes its recorded replacement forward. It validates full file sets, profiles, receipts, hashes, symlinks and original/backup/staged identities before mutation. Original and explicitly selected new state profiles are retained separately; installation repair checks both for active work. Cache repair takes installation/cache locks without stopping the daemon. Backups and task data remain untouched; no schema rollback or automatic process stop occurs. Completed repair is idempotent.

## Verification and limits

Behavior tests reconstruct the persistent states before, between and after renames for fresh installs, upgrades and caches with/without receipts. They cover modified/foreign files, unsafe profiles/symlinks, locks, legacy receipts, retained history and repeated recovery. An actual CLI repair test restores a missing installation and verifies unchanged SQLite bytes and stdio MCP loading afterward. These are deterministic filesystem fault scenarios, not physical power-loss tests.

Each directory is independently recoverable; marketplace/configuration writes, multiple selected caches and uninstall are not one transaction. Retry the original bootstrap/update after repair to complete remaining work. Before journal publication the original stays intact, but interrupted staging can leave unreferenced files. Old unjournaled interruptions and corrupted/missing operation artifacts require inspection. Never scan/delete caches or guess a backup. Full ENOSPC/power-loss qualification remains pending.

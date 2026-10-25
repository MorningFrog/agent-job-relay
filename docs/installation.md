# Local development installation and removal

This is a source-build Linux development milestone. No package is published.
Use a Rust stable/C toolchain. `cargo build --release --locked` creates the
single executable `target/release/agent-job-relay`; `cargo run -- --help` works.
The verified machine uses Rust 1.99.1; package MSRV 1.89 is not separately tested.
See the READMEs for a complete isolated quick start.

Optional isolated installation, verified without global PATH changes:

```bash
cargo install --path . --locked --root "$PWD/.tmp/install"
./.tmp/install/bin/agent-job-relay --version
cargo uninstall --root "$PWD/.tmp/install" agent-job-relay
```

Do not put state inside the installed binary directory. CLI `--state-dir`
(absolute) takes precedence over AJR_STATE_DIR, then XDG/HOME defaults. Every
component must agree on the state root. A foreground `daemon` owns the private
socket; no system service or login autostart is installed. Run `mcp` as a client
subprocess, not manually in the daemon terminal.

`setup codex --dry-run` previews the default Codex path; `--config` allows an
explicit test/client file. `setup json --config <absolute-file> --dry-run`
supports mcpServers JSON. Only explicit `--apply` writes, with an ownership
receipt and backup. The binary's current absolute path is inserted. The stdio
MCP entry points to the same state root. Restart the client after configuration.
ChatGPT private settings are not guessed. For hosts that require their own
registration UI, use the generated command/args and official client procedure.

Before removal: list jobs, stop desired jobs, Ctrl-C the foreground daemon,
then preview/apply `integration remove <client> --config <file>`. This removes
only an unchanged entry owned by AgentJobRelay. It does not delete job data or
stop already-running runners. Removing a development install with cargo uninstall
removes its executable and Cargo receipt, not state. For a manually copied binary,
remove only that known file after checking background processes.

There is no managed service remover, production installer or `data purge`
command. Normal future uninstall must preserve history/logs/config; destructive
cleanup will be a separate explicit operation. Never delete Vanth data or unknown
state directories. See TODO.md for every planned package/channel/uninstall test.

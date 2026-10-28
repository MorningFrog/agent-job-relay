#!/usr/bin/env node
// Match the online ChatGPT/Codex plugin unless another host is explicit.
if (!["codex", "opencode"].includes(process.argv[2])) process.argv.splice(2, 0, "codex");
await import("./cli.mjs");

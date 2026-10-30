// Behavioral public-CLI fixtures. No real host configuration or registry writes.
import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { inspectMarketplace, updateMarketplace } from "../packages/runtime/update.mjs";

function marketplaceFixture(t) {
  const temporary = fileURLToPath(new URL("../.tmp/", import.meta.url));
  mkdirSync(temporary, { recursive: true });
  const root = mkdtempSync(join(temporary, "marketplace-update-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const git = (...args) => execFileSync("git", ["-C", root, ...args], { encoding: "utf8" }).trim();
  git("init", "--quiet");
  const descriptor = join(root, "plugins/chatgpt/.codex-plugin/plugin.json");
  mkdirSync(dirname(descriptor), { recursive: true });
  writeFileSync(descriptor, JSON.stringify({ version: "0.1.14" }));
  git("add", ".");
  git("-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "-c", "commit.gpgsign=false", "commit", "--quiet", "-m", "test: seed marketplace fixture");
  git("-c", "tag.gpgsign=false", "tag", "v0.1.14");
  const source = "https://github.com/MorningFrog/agent-job-relay.git";
  const record = { source_type: "git", source, ref_name: "v0.1.14", sparse_paths: [], revision: git("rev-parse", "HEAD") };
  const recordPath = join(root, ".codex-marketplace-install.json");
  writeFileSync(recordPath, JSON.stringify(record));
  const run = async (program, args, { raw = false } = {}) => {
    if (args[0] === "plugin") return JSON.stringify({ marketplaces: [{ name: "agent-job-relay", root, marketplaceSource: { sourceType: "git", source } }] });
    const output = execFileSync(program, args, { encoding: "utf8" });
    return raw ? output : output.trim();
  };
  return { root, git, descriptor, record, recordPath, inspect: () => inspectMarketplace("codex", run) };
}

function host(version, enabled = true) {
  const state = { version, enabled, calls: [], failAdd: false, loseAck: false };
  state.run = async (_program, args) => {
    state.calls.push(args);
    if (args[1] === "list") return JSON.stringify({ installed: state.version ? [{ pluginId: "agent-job-relay@agent-job-relay", version: state.version, enabled: state.enabled }] : [] });
    if (args[1] === "marketplace") {
      if (args[2] === "add" && state.failAdd) throw new Error("injected fetch failure");
      return "{}";
    }
    if (args[1] === "add") {
      state.version = "0.1.14";
      if (state.loseAck) { state.loseAck = false; throw new Error("lost host acknowledgement"); }
      return JSON.stringify({ pluginId: "agent-job-relay@agent-job-relay", version: state.version });
    }
    throw new Error("unexpected host operation");
  };
  return state;
}

test("first installation and an unchanged retry use exact tag/plugin without repeated installation", async () => {
  const h = host(null), stages = [];
  await updateMarketplace("codex", "0.1.14", null, undefined, stage => stages.push(stage), h.run);
  assert(h.calls.some(args => args.includes("v0.1.14")));
  assert.equal(h.calls.filter(args => args[1] === "add").length, 1);
  await updateMarketplace("codex", "0.1.14", { version: "0.1.14" }, true, () => {}, h.run);
  assert.equal(h.calls.filter(args => args[1] === "add").length, 1);
  assert.equal(stages.at(-1), "plugin_verified");
});

test("partial marketplace replacement finishes forward without repeating removal", async () => {
  const h = host("0.1.13"), stages = [];
  h.failAdd = true;
  await assert.rejects(updateMarketplace("codex", "0.1.14", { version: "0.1.13" }, true, stage => stages.push(stage), h.run), /fetch failure/);
  assert(stages.includes("marketplace_removed"));
  h.failAdd = false;
  await updateMarketplace("codex", "0.1.14", null, true, () => {}, h.run);
  assert.equal(h.calls.filter(args => args[2] === "remove").length, 1);
  assert.equal(h.version, "0.1.14");
});

test("lost install acknowledgement is reconciled by public reads without replay", async () => {
  const h = host("0.1.13");
  h.loseAck = true;
  await assert.rejects(updateMarketplace("codex", "0.1.14", { version: "0.1.14" }, true, () => {}, h.run), /lost host acknowledgement/);
  await updateMarketplace("codex", "0.1.14", { version: "0.1.14" }, true, () => {}, h.run);
  assert.equal(h.calls.filter(args => args[1] === "add").length, 1);
});

test("disabled outdated plugins are preserved before any host mutation", async () => {
  const h = host("0.1.13", false);
  await assert.rejects(updateMarketplace("codex", "0.1.14", { version: "0.1.13" }, false, () => {}, h.run), /disabled/);
  assert(h.calls.every(args => args[1] === "list"));
});

test("foreign and modified marketplaces fail before replacement", async () => {
  const market = { name: "agent-job-relay", root: "fixture", marketplaceSource: { sourceType: "local", source: "fixture" } };
  await assert.rejects(inspectMarketplace("codex", async () => JSON.stringify({ marketplaces: [market] })), /local or foreign/);
  market.marketplaceSource = { sourceType: "git", source: "https://github.com/MorningFrog/agent-job-relay.git" };
  await assert.rejects(inspectMarketplace("codex", async (_program, args) => args[0] === "plugin" ? JSON.stringify({ marketplaces: [market] }) : " M plugins/chatgpt/relay"), /local changes/);
});

test("clean development checkouts cannot masquerade as matching published versions", async () => {
  const market = { name: "agent-job-relay", root: "fixture", marketplaceSource: { sourceType: "git", source: "https://github.com/MorningFrog/agent-job-relay.git" } };
  const run = async (_program, args) => {
    if (args[0] === "plugin") return JSON.stringify({ marketplaces: [market] });
    if (args.includes("show")) return JSON.stringify({ version: "0.1.14" });
    return "";
  };
  await assert.rejects(inspectMarketplace("codex", run), /matching release tag/);
});

test("Codex's exact untracked installation record permits a released checkout without altering it", async t => {
  const f = marketplaceFixture(t);
  const original = readFileSync(f.recordPath);
  assert.deepEqual(await f.inspect(), { version: "0.1.14", root: f.root });
  assert.deepEqual(readFileSync(f.recordPath), original);
  assert.equal(f.git("status", "--porcelain=v1"), "?? .codex-marketplace-install.json");
  rmSync(f.recordPath);
  assert.deepEqual(await f.inspect(), { version: "0.1.14", root: f.root });
});

test("host metadata never hides tracked, staged or additional untracked user changes", async t => {
  const f = marketplaceFixture(t);
  for (const name of ["notes.txt", ".codex-marketplace-install.json.extra", "nested/.codex-marketplace-install.json", ...(process.platform === "win32" ? [] : ["notes\n?? .codex-marketplace-install.json"])]) {
    const path = join(f.root, name);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, "user content");
    await assert.rejects(f.inspect(), error => {
      assert(error.message.includes(JSON.stringify(f.root)));
      assert(error.message.includes(`untracked: ${JSON.stringify(name)}`));
      return true;
    });
    assert.equal(readFileSync(path, "utf8"), "user content");
    rmSync(path);
  }
  const descriptor = readFileSync(f.descriptor);
  writeFileSync(f.descriptor, "user content");
  await assert.rejects(f.inspect(), /modified in working tree: "plugins\/chatgpt\/\.codex-plugin\/plugin.json"/);
  assert.equal(readFileSync(f.descriptor, "utf8"), "user content");
  writeFileSync(f.descriptor, descriptor);
  f.git("add", ".codex-marketplace-install.json");
  await assert.rejects(f.inspect(), /added in index: "\.codex-marketplace-install.json"/);
});

test("dirty-check diagnostics show both rename paths and bound long change inventories", async t => {
  const f = marketplaceFixture(t);
  f.git("mv", "plugins/chatgpt/.codex-plugin/plugin.json", "plugins/chatgpt/.codex-plugin/renamed.json");
  await assert.rejects(f.inspect(), /renamed in index: "plugins\/chatgpt\/\.codex-plugin\/plugin.json" -> "plugins\/chatgpt\/\.codex-plugin\/renamed.json"/);
  for (let i = 0; i < 12; i++) writeFileSync(join(f.root, `user-${i}.txt`), "preserve me");
  await assert.rejects(f.inspect(), error => {
    assert.equal(error.message.split("\n").filter(line => line.startsWith("  - ")).length, 11);
    assert(error.message.includes("3 more changed paths"));
    assert(!error.message.includes('untracked: ".codex-marketplace-install.json"'));
    assert(error.message.includes("--cli-only"));
    return true;
  });
});

test("malformed, foreign, stale and sparse installation records remain protected", async t => {
  const f = marketplaceFixture(t);
  for (const [content, reason] of [["{", "not valid JSON"], ["null", "expected a JSON object"], [JSON.stringify({}), "source_type"], ...[
    [{ source_type: "local" }, "source_type"], [{ source: "https://example.invalid/foreign.git" }, "source"],
    [{ ref_name: "v0.1.13" }, "ref_name"], [{ revision: "0".repeat(40) }, "revision"],
    [{ sparse_paths: ["plugins/chatgpt"] }, "sparse_paths"], [{ sparse_paths: null }, "sparse_paths"],
    [{ user_notes: "preserve custom metadata" }, "unrecognized fields"],
  ].map(([override, reason]) => [JSON.stringify({ ...f.record, ...override }), reason]), [" ".repeat(65537), "65536-byte limit"]]) {
    writeFileSync(f.recordPath, content);
    await assert.rejects(f.inspect(), error => {
      assert(error.message.includes(JSON.stringify(f.recordPath)));
      assert(error.message.includes(reason));
      assert(!error.message.includes("preserve custom metadata"));
      return true;
    });
    assert.equal(readFileSync(f.recordPath, "utf8"), content);
  }
  writeFileSync(f.recordPath, JSON.stringify(f.record));
  f.git("tag", "--delete", "v0.1.14");
  await assert.rejects(f.inspect(), /matching release tag/);
});

test("an installation-record symlink cannot authorize marketplace replacement", { skip: process.platform === "win32" }, async t => {
  const f = marketplaceFixture(t);
  const target = join(f.root, ".git", "fixture-record.json");
  writeFileSync(target, JSON.stringify(f.record));
  rmSync(f.recordPath);
  symlinkSync(target, f.recordPath);
  await assert.rejects(f.inspect(), /installation record.*symbolic links are not accepted/);
  assert.deepEqual(JSON.parse(readFileSync(target, "utf8")), f.record);
});

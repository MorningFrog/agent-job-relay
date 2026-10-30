// Behavioral public-CLI fixtures. No real host configuration or registry writes.
import { test } from "node:test";
import assert from "node:assert/strict";
import { inspectMarketplace, updateMarketplace } from "../packages/runtime/update.mjs";

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

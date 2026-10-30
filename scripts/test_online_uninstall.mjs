// Public-host removal fixtures; no real application state or npm writes.
import { test } from "node:test";
import assert from "node:assert/strict";
import { removeOnlinePlugin } from "../packages/runtime/update.mjs";

function host({ shared = false, foreign = false } = {}) {
  const source = { sourceType: "git", source: foreign ? "https://example.invalid/foreign.git" : "https://github.com/MorningFrog/agent-job-relay.git" };
  const plugin = { pluginId: "agent-job-relay@agent-job-relay", marketplaceName: "agent-job-relay", version: "0.1.14", enabled: true, marketplaceSource: source };
  const state = { plugin, marketplace: { version: "0.1.14", root: "fixture" }, shared, calls: [], losePluginAck: false, loseMarketAck: false };
  state.snapshot = () => ({ plugin: state.plugin, marketplace: state.marketplace, shared: state.shared });
  state.run = async (_program, args) => {
    state.calls.push(args);
    if (args[0] === "-C") {
      if (args.includes("show")) return JSON.stringify({ version: "0.1.14" });
      return args.includes("tag") ? "v0.1.14" : "";
    }
    if (args[1] === "marketplace") {
      if (args[2] === "list") return JSON.stringify({ marketplaces: state.marketplace ? [{ name: "agent-job-relay", root: "fixture", marketplaceSource: { sourceType: "git", source: "https://github.com/MorningFrog/agent-job-relay.git" } }] : [] });
      assert.equal(args[2], "remove");
      state.marketplace = null;
      if (state.loseMarketAck) { state.loseMarketAck = false; throw new Error("lost marketplace acknowledgement"); }
      return "{}";
    }
    if (args[1] === "list") return JSON.stringify({ installed: [...(state.plugin ? [state.plugin] : []), ...(state.shared ? [{ pluginId: "other@agent-job-relay", marketplaceName: "agent-job-relay" }] : [])] });
    assert.equal(args[1], "remove");
    state.plugin = undefined;
    if (state.losePluginAck) { state.losePluginAck = false; throw new Error("lost plugin acknowledgement"); }
    return "{}";
  };
  return state;
}

test("removal deletes the exact plugin before its unshared marketplace", async () => {
  const h = host(), stages = [];
  const result = await removeOnlinePlugin("codex", h.snapshot(), stage => stages.push(stage), h.run);
  assert.equal(result.marketplace_retained, false);
  const writes = h.calls.filter(args => args.includes("remove"));
  assert.deepEqual(writes, [["plugin", "remove", "agent-job-relay@agent-job-relay", "--json"], ["plugin", "marketplace", "remove", "agent-job-relay"]]);
  assert.equal(stages.at(-1), "host_removed");
});

test("other installed plugins retain their shared marketplace", async () => {
  const h = host({ shared: true });
  const result = await removeOnlinePlugin("codex", h.snapshot(), () => {}, h.run);
  assert.equal(result.marketplace_retained, true);
  assert(!h.calls.some(args => args[1] === "marketplace" && args[2] === "remove"));
});

test("uncertain plugin and marketplace removals reconcile without repeating mutations", async () => {
  for (const [flag, message] of [["losePluginAck", /lost plugin/], ["loseMarketAck", /lost marketplace/]]) {
    const h = host();
    h[flag] = true;
    await assert.rejects(removeOnlinePlugin("codex", h.snapshot(), () => {}, h.run), message);
    await removeOnlinePlugin("codex", h.snapshot(), () => {}, h.run);
    assert.equal(h.calls.filter(args => args[1] === "remove").length, 1);
    assert.equal(h.calls.filter(args => args[2] === "remove").length, 1);
  }
});

test("a foreign cached plugin is retained before any host mutation", async () => {
  const h = host({ foreign: true });
  await assert.rejects(removeOnlinePlugin("codex", h.snapshot(), () => {}, h.run), /unknown\/local\/foreign/);
  assert(!h.calls.some(args => args.includes("remove")));
});

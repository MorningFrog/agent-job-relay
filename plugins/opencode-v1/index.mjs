import { tool } from "@opencode-ai/plugin";
import { createBridge } from "./bridge.mjs";
import { resolveExecutable, defaultStateDir } from "agent-job-relay-runtime";

export const AgentJobRelay = async ({ client, directory }, options = {}) => {
  const executable = options.executable ?? resolveExecutable();
  const stateDir = options.stateDir ?? defaultStateDir("opencode");
  const bridge = await createBridge({ client, directory, executable, stateDir });
  return {
    config: async (config) => {
      config.mcp ??= {};
      if (config.mcp.agent_job_relay) { await bridge.close();throw new Error("MCP name agent_job_relay is already configured; preserve/review it before enabling this plugin"); }
      config.mcp.agent_job_relay = { type: "local", command: [executable, "--state-dir", stateDir, "mcp", "--shared-host"], enabled: true };
    },
    "tool.execute.before": async (input, output) => {
      if (!["agent_job_relay_job_start", "agent_job_relay_job_start_batch"].includes(input.tool)) return;
      const specs = input.tool === "agent_job_relay_job_start_batch" ? output.args?.specs : [output.args?.spec];
      if (!Array.isArray(specs) || specs.length < 1 || specs.length > 16 || specs.some(spec => !spec || typeof spec !== "object" || Array.isArray(spec))) throw new Error("AgentJobRelay requires explicit start specifications (batch: 1..16)");
      const key = await bridge.bind(input.sessionID);
      for (const spec of specs) spec.session = key;
      // Caller identity always comes from this hook, including model-supplied bindings.
      delete output.args.binding;
      output.args.wake_me = false;
    },
    "experimental.chat.system.transform": async (_input, output) => {
      output.system.push("AgentJobRelay binds this OpenCode caller on submission. Omit session, binding and wake_me. Use instructions delegate for submission guidance or feedback for rule syntax. Follow the receipt and yield; use job_inspect for needed evidence. For recovery use relay_recover.");
    },
    tool: {
      relay_recover: tool({
        description: "Preview same-chat recovery after an OpenCode plugin/server restart; apply only when the user requests restoration. Never reruns commands.",
        args: { apply: tool.schema.boolean().default(false), after: tool.schema.number().int().min(0).default(0) },
        async execute({ apply, after }, context) {
          const key = await bridge.bind(context.sessionID);
          return JSON.stringify(await bridge.request({ op: "recover_session", session_key: key, apply, after, limit: 20 }));
        },
      }),
    },
    event: async ({ event }) => { if (event.type === "server.instance.disposed" && event.properties.directory === directory) await bridge.close(); },
  };
};

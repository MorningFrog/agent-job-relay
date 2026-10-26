import { readFileSync, lstatSync, accessSync, constants } from "node:fs";
import { createHash } from "node:crypto";
import { dirname, join, isAbsolute } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url));

export function resolveExecutable() {
  if (process.platform !== "linux" || process.arch !== "x64") {
    throw new Error("AgentJobRelay currently supports Linux x86_64 only; macOS/Windows/ARM are not implemented");
  }
  const manifest = JSON.parse(readFileSync(join(root, "native/manifest.json"), "utf8"));
  const metadata = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
  if (manifest.format !== 1 || manifest.platform !== "linux-x64" || manifest.version !== metadata.version || !/^[0-9a-f]{64}$/.test(manifest.sha256) || !/^\d+\.\d+$/.test(manifest.minimumGlibc)) {
    throw new Error("Invalid AgentJobRelay native manifest; reinstall the same trusted package");
  }
  const libc = process.report?.getReport?.().header.glibcVersionRuntime;
  const [major, minor] = (libc ?? "").split(".").map(Number);
  const [requiredMajor, requiredMinor] = manifest.minimumGlibc.split(".").map(Number);
  if (!libc || major < requiredMajor || major === requiredMajor && minor < requiredMinor) {
    throw new Error(`AgentJobRelay requires glibc >=${manifest.minimumGlibc}; musl/unknown libc is unsupported`);
  }
  const executable = join(root, "native/linux-x64/agent-job-relay");
  const stat = lstatSync(executable);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > 128 * 1024 * 1024 || createHash("sha256").update(readFileSync(executable)).digest("hex") !== manifest.sha256) {
    throw new Error("AgentJobRelay native payload failed integrity validation; nothing was executed");
  }
  accessSync(executable, constants.X_OK);
  return executable;
}

// Host profiles are stable across package/cache replacements and never carry caller identity.
export function defaultStateDir(host) {
  if (!["codex", "opencode"].includes(host)) throw new Error("Unknown AgentJobRelay host profile");
  const base = process.env.XDG_STATE_HOME || (process.env.HOME && join(process.env.HOME, ".local/state"));
  const state = process.env.AJR_STATE_DIR || (base && join(base, "agent-job-relay", host));
  if (!state || !isAbsolute(state)) throw new Error("AgentJobRelay state path must be absolute; set AJR_STATE_DIR or an absolute HOME/XDG_STATE_HOME");
  return state;
}

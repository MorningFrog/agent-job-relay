import { readFileSync, lstatSync, accessSync, constants } from "node:fs";
import { createHash } from "node:crypto";
import { dirname, join, isAbsolute } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url));

export function resolveExecutable() {
  const platform = `${process.platform}-${process.arch}`;
  if (!["linux-x64", "win32-x64"].includes(platform)) {
    throw new Error("AgentJobRelay supports Linux x86_64/glibc and Windows x86_64; macOS/ARM are not implemented");
  }
  const manifest = JSON.parse(readFileSync(join(root, "native/manifest.json"), "utf8"));
  const metadata = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
  const target = manifest.format === 2 ? manifest.targets?.[platform] : manifest.format === 1 && manifest.platform === platform ? manifest : undefined;
  if (manifest.version !== metadata.version || !target || !/^[0-9a-f]{64}$/.test(target.sha256)) {
    throw new Error("Invalid or missing AgentJobRelay native target; reinstall the matching trusted package");
  }
  if (process.platform === "linux") {
    if (!/^\d+\.\d+$/.test(target.minimumGlibc)) throw new Error("Invalid native glibc requirement");
    const libc = process.report?.getReport?.().header.glibcVersionRuntime;
    const [major, minor] = (libc ?? "").split(".").map(Number);
    const [requiredMajor, requiredMinor] = target.minimumGlibc.split(".").map(Number);
    if (!libc || major < requiredMajor || major === requiredMajor && minor < requiredMinor) {
      throw new Error(`AgentJobRelay requires glibc >=${target.minimumGlibc}; musl/unknown libc is unsupported`);
    }
  }
  const executable = join(root, "native", platform, process.platform === "win32" ? "agent-job-relay.exe" : "agent-job-relay");
  const stat = lstatSync(executable);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > 128 * 1024 * 1024 || createHash("sha256").update(readFileSync(executable)).digest("hex") !== target.sha256) {
    throw new Error("AgentJobRelay native payload failed integrity validation; nothing was executed");
  }
  accessSync(executable, constants.X_OK);
  return executable;
}

export function defaultStateDir(host) {
  if (!["codex", "opencode"].includes(host)) throw new Error("Unknown AgentJobRelay host profile");
  const linuxState = process.env.XDG_STATE_HOME || (process.env.HOME && join(process.env.HOME, ".local/state"));
  const base = process.platform === "win32"
    ? process.env.LOCALAPPDATA && join(process.env.LOCALAPPDATA, "AgentJobRelay", "profiles")
    : linuxState && join(linuxState, "agent-job-relay");
  const state = process.env.AJR_STATE_DIR || (base && join(base, host));
  if (!state || !isAbsolute(state)) throw new Error("AgentJobRelay state path must be absolute; set AJR_STATE_DIR or the platform's state base");
  return state;
}

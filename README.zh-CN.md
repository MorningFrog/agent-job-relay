<div align="center"> 
<img src="./assets/icons/agent-job-relay.svg" alt="icon" width="200"/>
<h1>AgentJobRelay</h1>

[English](README.md) | 简体中文

Run durable background commands and route completion events to explicitly registered AI agent sessions.

持久执行后台命令, 将完成事件可靠地路由到明确注册的 AI Agent 会话.
</div>

面向单个可信本机用户的 Rust 原生 CLI、独立 Runner 和 stdio MCP Server。
**v0.1.0 是源码构建的开发里程碑，已在 Linux x86_64 验证。** 当前没有安装包或包管理器发行版。

## 当前能力

| 能力 | 实际证据 |
|---|---|
| 显式 argv 或主动选择 Shell、cwd、超时、幂等提交 | Linux 真实进程测试 |
| 独立 Daemon/Runner、SQLite Job/Run/Event、stdout/stderr 有界保存 | Linux 测试，包含 Daemon 崩溃与重启 |
| 进程组取消、收养子进程回收 | Linux 测试；主动逃离进程组的守护进程不在隔离保证内 |
| 标准 stdio MCP，客户端退出不影响任务 | 真实握手、工具发现与执行测试 |
| 会话、租期、精确路由、事务 Outbox、重试/TTL/dead letter | 持久化与故障自动化测试 |
| Codex CLI 集成 | MCP 配置与通知收件箱；**不能自动恢复 Agent** |
| Desktop 唤醒 | 实验性 Unix Socket 适配器；自动化假宿主测试，以及 ChatGPT Desktop 26.1021.22027 / Ubuntu 26.04 上**用户确认通过的真实端到端测试**，覆盖显式登记和 CLI `--wake` 首次启动、复用、空闲 Daemon 重启 |
| setup、doctor、移除所属 MCP 项 | 隔离配置测试；没有修改真实用户设置 |
| Windows/macOS 运行时 | **尚未实现/验证**，明确返回不支持 |

任务完成、`notification_recorded`、`accepted_by_host` 与观察到 Agent 真正继续执行是不同状态。
当前没有适配器能够自动确认 Agent 已恢复。Desktop 的 `can_resume` 表示配置的请求机制，不能视为真实端到端验证。
用户确认通过的实测及本地证据核对见[真实端到端验证记录](docs/live-e2e.md)。CLI 快捷入口三轮均在无人发消息时自动续接，对照聊天未收到错误通知。任务运行中 Daemon 中断、Desktop 重启和重新绑定仍需单独验证。

## 一分钟源码快速开始

需要 Rust stable（包声明的最低版本为 1.89）、Cargo 和用于编译内置 SQLite 的 C 工具链。
运行时不依赖 Python、Node、Go、Docker 或外部数据库。从项目根目录执行：

```bash
cargo build --release --locked
export AJR_STATE_DIR="$PWD/.tmp/demo/state"
./target/release/agent-job-relay start -- /bin/echo "hello from the runner"
./target/release/agent-job-relay doctor
```

**首次提交任务会自动启动独立 Daemon**，不用额外保留终端，也不安装系统服务。
返回值包含 job.job_id 和 job.run_id，将实际 job_id 设置为 JOB_ID 后查询：

```bash
./target/release/agent-job-relay status "$JOB_ID"
./target/release/agent-job-relay logs "$JOB_ID"
./target/release/agent-job-relay events "$JOB_ID"
```

在 Desktop Agent 的终端工具中，只需一个选项即可绑定当前会话唤醒：

```bash
./target/release/agent-job-relay start --wake -- cargo check
```

程序只读取当前进程继承的 CODEX_THREAD_ID 和 CODEX_APP_TOOLS_PIPE_PATH，自动登记/刷新、探测宿主并绑定任务。
不必手工复制 thread ID、socket 或注册 key。`--wake` 表示显式选择实验性 Desktop 适配器。
缺少可靠身份或宿主能力时，任务不会启动，并返回如何从正确宿主上下文使用的提示；不会回退到收件箱或其他会话。

```bash
./target/release/agent-job-relay session connect  # 可选的连接诊断
./target/release/agent-job-relay daemon stop     # 保留 Runner 和状态数据
```

`daemon` 仍可用于前台诊断，Ctrl-C 不停止独立 Runner；下次提交会重新启动已停止的 Daemon。
没有登录自启动项。默认状态目录为 `$XDG_STATE_HOME/agent-job-relay`，否则使用
`$HOME/.local/state/agent-job-relay`；可用 `--state-dir /绝对路径` 覆盖。
CLI、MCP 必须使用同一目录，已有目录须属于当前用户且权限为 0700，Socket 路径短于 104 字节。

## CLI 示例

```bash
./target/release/agent-job-relay start --timeout 60 --idempotency-key build-001 -- cargo check
./target/release/agent-job-relay start --shell 'printf "out\n"; printf "err\n" >&2'
./target/release/agent-job-relay logs "$JOB_ID" --stream stderr --cursor 0 --limit 8192
./target/release/agent-job-relay logs "$JOB_ID" --follow
./target/release/agent-job-relay wait "$JOB_ID" --after 0 --seconds 10
./target/release/agent-job-relay stop "$JOB_ID"
```

`--` 分隔 CLI 选项与程序参数；Shell 必须显式选择。CLI 未指定 cwd 时使用当前目录，MCP 则必须提供绝对 cwd。
任务继承 **Daemon 的环境变量**，可通过 MCP/request 的 `env` 提供显式覆盖值。覆盖值会保存在私有数据库，但状态查询不回显。

列表、事件、投递结果返回 `next_cursor`，下一页用 `--after`。日志游标是字节偏移，不能用字符数替代。
日志查询采用有损 UTF-8，原始字节按 stdout/stderr 分别保存，达到各 16 MiB 后继续排空并丢弃额外输出。
`--follow` 面向人工查看，Ctrl-C 只停止查看。`wait` 在新事件（包括 `started`）、终态或超时时返回，可能早于任务完成。
长任务应使用具有相应能力的目标，或稍后查询；不要让 Agent 高频循环轮询。

最多允许 8 个活动任务。相同幂等键及完全相同参数返回原任务，参数不同则报冲突；有意重跑应使用新键。
`stop` 幂等，已经完成的任务不会改成取消。

## MCP 配置与安全移除

默认只预览，`--apply` 表示调用者明确授权写入。先使用隔离文件，以下命令不会配置真实 Codex：

```bash
./target/release/agent-job-relay setup codex --config "$PWD/.tmp/demo/codex.toml" --dry-run
./target/release/agent-job-relay setup codex --config "$PWD/.tmp/demo/codex.toml" --apply
./target/release/agent-job-relay doctor --client codex --config "$PWD/.tmp/demo/codex.toml"
./target/release/agent-job-relay integration remove codex --config "$PWD/.tmp/demo/codex.toml" --dry-run
./target/release/agent-job-relay integration remove codex --config "$PWD/.tmp/demo/codex.toml" --apply
```

配置真实 Codex 时，选择实际 `--config` 路径；省略后使用 `$CODEX_HOME/config.toml` 或 `$HOME/.codex/config.toml`。
先查看 `setup codex --dry-run`，再自行选择 `--apply`；修改后重启 MCP 客户端。
生成项指向**当前可执行文件**及状态目录，移动二进制后应重新 setup。setup 不安装系统服务，提交任务时会自动启动 Daemon。

使用 `mcpServers` JSON 格式的通用客户端可以执行：

```bash
./target/release/agent-job-relay setup json --config "$PWD/.tmp/demo/mcp.json" --dry-run
./target/release/agent-job-relay setup json --config "$PWD/.tmp/demo/mcp.json" --apply
./target/release/agent-job-relay integration remove json --config "$PWD/.tmp/demo/mcp.json" --apply
```

不会猜测 ChatGPT Desktop 的配置位置或不受支持的格式。需要手动配置的客户端可以参考生成项结构：

```json
{
  "command": "/absolute/path/to/agent-job-relay",
  "args": ["--state-dir", "/absolute/private/state", "mcp"]
}
```

只修改本项目拥有的 `agent-job-relay` MCP 项，并在旁边保存私有的所有权凭据、锁和唯一备份。
保留其他配置，拒绝覆盖或移除已被用户改动、或没有所有权凭据的条目。
备份可能含有秘密，请保持私有。恢复备份前，应检查客户端是否已有更新的设置。
移除集成会保留全部任务历史、日志及状态配置。配置与凭据各自原子写入，但不是跨文件事务；中断 setup 后可能需要核对凭据和备份，程序会保守拒绝接管。

`agent-job-relay mcp` 的 stdout 仅用于 MCP，诊断写入 stderr。
工具包括 `job_start/status/list/logs/events/stop/wait/doctor`、自动连接、会话注册、探测、失效、收件箱、目标添加/重新绑定，以及投递列表、尝试记录和重试。
`job_start` 参数形如 `{"spec":{"command":{"kind":"exec","program":"/bin/echo",
"args":["hello"]},"cwd":"/absolute/workdir"}}`。准确 schema 和有界结果见[协议文档](docs/protocol.md)。

## 会话与唤醒

日常 MCP 使用直接在启动参数中选择唤醒：

```json
{
  "wake_me": true,
  "spec": {
    "command": { "kind": "exec", "program": "cargo", "args": ["check"] },
    "cwd": "/absolute/workdir"
  }
}
```

`job_start` 自动启动 Daemon；wake_me=true 还会根据 MCP 进程继承的 Desktop 上下文自动登记、刷新和探测。
之后 Agent 可结束当前回合，等待任务完成消息。`job_connect_session` 是可选的无参数诊断工具，正常启动不必预先调用。
wake_me 不应与显式 spec.session 同时使用，CLI 的 `--wake` 和 `--session` 也互斥。

自动绑定要求每会话独立的 MCP 进程实际获得上述两个宿主环境变量。共享 MCP 进程无法判断哪个聊天发起了调用。
如果变量没有传给 MCP，让 Agent 从当前 Desktop 的终端工具直接执行 `start --wake`，或显式提供已授权的 spec.session。
正常终端工具路径无需人工复制任何身份参数。

自动登记租期为 24 小时，用 Socket 指纹标识宿主实例；每次请求唤醒都会刷新。
端点替换会产生新绑定，不会迁移已有任务。超过该租期的任务需要显式刷新或重新绑定。
宿主接受请求仍不代表 Agent 已恢复。CLI 快捷入口的首次启动、复用和空闲 Daemon 重启已在记录的 Desktop 版本与系统上通过用户确认的真实测试。MCP `wake_me=true` 已有自动化假宿主覆盖，尚未单独验证其真实宿主路径。

高级手工登记仍保留，用于其他宿主或诊断。本地收件箱演示使用显式身份注册：

```bash
./target/release/agent-job-relay session register --agent codex_cli --session-id demo-thread --instance-id demo-instance --host-id local
```

将返回的 `session.key` 设置为 `SESSION_KEY`；它不同于逻辑 session_id。真实集成须使用实际宿主与会话身份：

```bash
./target/release/agent-job-relay session probe "$SESSION_KEY"
./target/release/agent-job-relay start --session "$SESSION_KEY" -- /bin/echo notified
./target/release/agent-job-relay inbox "$SESSION_KEY"
./target/release/agent-job-relay delivery list --job-id "$JOB_ID"
./target/release/agent-job-relay delivery attempts "$DELIVERY_ID"
./target/release/agent-job-relay delivery retry "$DELIVERY_ID"
```

收件箱是持久化通知存储，**不会自动恢复 Codex**。默认租期为一小时，可用 `--lease-seconds` 设置，最多 86400 秒。
用相同的显式身份重新注册即可刷新；新宿主实例使用新 key，已有任务不会自动切换到“最近的会话”。
用 `target add JOB_ID SESSION_KEY --replay` 显式补投有效期内的终态事件，或在替换宿主后执行
`target rebind TARGET_ID NEW_SESSION_KEY`。

Desktop 还必须显式提供 `--endpoint`、`--caller-thread-id`、`--experimental`，仅可使用有权控制的会话和端点。
私有 API 可能改变，Windows Named Pipe 尚未实现。[人工端到端测试流程](docs/testing.md)说明如何验证，不把假宿主测试当作真实 Agent 执行。
Desktop 失败不会启动另一个 CLI 会话。稳定投递 ID 供接收方去重，但确认丢失仍可能产生重复宿主副作用。

## 恢复、安全与卸载

SQLite 在启动前保存执行意图；每个 Runner 持有独占运行锁，只能认领一次。
启动结果不明时标记 `orphaned`，不会静默重复执行。Daemon 停止期间，Runner 仍可保存终态、事件和 Outbox。
Runner 或启动身份消失不代表成功。关机停止进程，持久状态只支持恢复分类，不代表跨断电继续执行。

命令拥有当前用户的系统权限。当前没有执行沙箱、CPU/内存配额、日志秘密掩码，也无法隔离故意逃离进程组的进程。
不要把不可信输出当成高优先级 Agent 指令。剩余边界见[安全说明](docs/security.md)与[限制](docs/limitations.md)。

可选的隔离开发安装，不改全局 PATH：

```bash
cargo install --path . --locked --root "$PWD/.tmp/install"
./.tmp/install/bin/agent-job-relay --version
cargo uninstall --root "$PWD/.tmp/install" agent-job-relay
```

移除或替换二进制前，先检查活动任务，按需停止任务，并对相同状态目录执行 `agent-job-relay daemon stop`，或用 Ctrl-C 退出前台 Daemon。
卸载软件、移除 MCP 集成、移除系统服务和删除用户数据是不同操作。
手动复制的二进制也应在检查进程后只删除该文件。**普通移除保留状态数据。**
当前没有 `data purge` 或系统卸载器，不要删除未知目录或其他项目数据。未来安装器卸载也必须默认保留历史、日志和用户配置。

## 常见问题与开发验证

- **Daemon unavailable：** 提交任务会自动启动相同状态目录的 Daemon；`doctor` 只检查，不会主动启动。
  自动启动失败时，用前台 `daemon` 查看诊断；没有安装系统服务。
- **看不到 MCP：** 预览并应用正确客户端配置，再重启客户端。
  `doctor --client codex --config ...` 检查所属配置，不代表已验证真实 MCP 连接。
- **Agent 没有唤醒：** 在当前 Desktop 上下文中使用 `start --wake` 或 wake_me=true，`session connect` 可诊断连接。
  对已有任务检查会话租期、投递状态和尝试记录。
  Inbox 不具备恢复能力；Desktop 接受请求不代表 Agent 已执行。
- **orphaned：** 先审计外部副作用，再决定是否以新任务重跑。不会自动重放；逃逸的孙进程可能需要人工检查。
- **权限或 Socket 错误：** 使用当前用户拥有、0700 权限且路径更短的状态目录；不要放宽权限或复用 Vanth 数据。
- **拒绝移除配置：** 条目被改动或缺少所有权凭据，需核对备份及设置；其他 MCP 项及任务数据保持完整。

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release --locked
```

测试在 `.tmp/` 下创建隔离状态，Linux 使用真实进程，Desktop 私有传输使用模拟宿主。
CI 配置包含 Linux 完整测试与 Windows/macOS 编译及库检查；没有运行远程 CI，也没有本机验证另外两个系统。
更多内容见[架构](docs/architecture.md)、[测试](docs/testing.md)、[实施状态](docs/IMPLEMENTATION_STATUS.md)、
[参考审查](docs/reference-audit.md)及[变更记录](CHANGELOG.md)。

## 路线图 / 延后发行

`.deb`/APT、`.rpm`/DNF、`.AppImage`、Windows `.exe`/WinGet、macOS `.dmg`/Homebrew、
Flatpak/Flathub、GitHub Releases 二进制、可选 PyPI Wheel/`uv tool`、签名、公证和 GitHub Actions 软件仓库发布
均为 **Planned / Not implemented**。没有虚构安装链接或已经发布的包命令。
[Distribution & Packaging TODO](docs/TODO.md)列出优先级、安装/升级/卸载语义、限制、测试标准及需单独授权的仓库发布。

MIT 许可。项目独立实现，只读研究 Vanth 的行为需求，没有复制实现，也没有创建或推送远程仓库。

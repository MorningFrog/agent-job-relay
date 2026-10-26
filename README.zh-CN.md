<div align="center"> 
<img src="./assets/icons/agent-job-relay.svg" alt="icon" width="200"/>
<h1>AgentJobRelay</h1>

[English](README.md) | 简体中文

Run durable background commands and route completion events to explicitly registered AI agent sessions.

持久执行后台命令, 将完成事件可靠地路由到明确注册的 AI Agent 会话.
</div>

面向单个可信本机用户的 Rust 原生 CLI、独立 Runner 和 stdio MCP Server。
**v0.1.2 是尚未发布的开发里程碑，已在 Linux x86_64 验证。**
交付方向统一转向 Agent Plugin，独立系统安装包及软件仓库计划已取消；开发阶段暂不发布到官方插件平台。

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
| 一次安装的本地 Desktop Plugin + MCP | 安装后的技能 + CLI 降级入口已完成真实 release 测试任务并唤醒同一聊天；修复后的 Codex CLI 0.164.0 加载器在隔离环境发现 19 个 MCP 工具；0.1.2 Desktop MCP 连通性已获用户确认；MCP 任务/唤醒路径待验收 |
| Windows/macOS 运行时 | **尚未实现/验证**，明确返回不支持 |

任务完成、`notification_recorded`、`accepted_by_host` 与观察到 Agent 真正继续执行是不同状态。
当前没有适配器能够自动确认 Agent 已恢复。Desktop 的 `can_resume` 表示配置的请求机制，不能视为真实端到端验证。
用户确认通过的实测及本地证据核对见[真实端到端验证记录](docs/live-e2e.md)。CLI 快捷入口三轮均在无人发消息时自动续接，对照聊天未收到错误通知。任务运行中 Daemon 中断、Desktop 重启和重新绑定仍需单独验证。

## 安装一次，从聊天直接使用

源码构建后，将原生程序和 Desktop 插件安装到稳定的私有目录。下面是用户主动选择的**个人安装**；没有 `--apply` 的命令只预览。开发测试使用仓库内隔离的 `.tmp/` 路径。

```bash
cargo build --release --locked
AJR_INSTALL_ROOT="$HOME/.local/share/agent-job-relay"
./target/release/agent-job-relay install --root "$AJR_INSTALL_ROOT"
./target/release/agent-job-relay install --root "$AJR_INSTALL_ROOT" --apply
"$AJR_INSTALL_ROOT/relay" setup plugin --dry-run
"$AJR_INSTALL_ROOT/relay" setup plugin --apply
"$AJR_INSTALL_ROOT/relay" doctor --client plugin --check-mcp
```

重启 Desktop，在 Plugins 中选择个人本地 marketplace（新建时名为 `AgentJobRelay Local`），安装 AgentJobRelay。打开新的本地聊天，按需选择插件，然后直接说：

> 后台执行这个项目的测试，完成后告诉我结果。

日常提示词无需工具路径、状态目录或会话 ID。插件技能取得当前聊天绑定后，在 MCP 可用时通过 MCP 提交，否则明确报告使用 CLI 降级入口。提交后结束回合，完成时读取结果；工作目录由已选项目确定。用户的 release 测试任务已加载安装后的技能，通过 CLI 降级入口完成并唤醒同一聊天。首次运行时错误及后续绕行情况见[真实测试记录](docs/live-e2e.md)。

修复后的本地程序包使用 `.codex-plugin/plugin.json` 和 `.mcp.json`，真实 Codex CLI 0.164.0 已在隔离环境发现全部 19 个 MCP 工具。用户按照宿主 `job_doctor` 流程确认更新后的 0.1.2 Desktop MCP 连通性测试通过；此次属于用户确认，未独立审阅新的工具调用记录。MCP 任务提交/唤醒和插件界面安装仍需独立证据；Desktop 唤醒仍是实验性机制。详见[安装说明](docs/installation.md)。

安装不修改 PATH、不配置系统服务、不公开发布插件。状态数据与程序目录独立，复制到插件缓存后仍使用同一状态配置。插件固定该配置，不受已导出 AJR_STATE_DIR 的影响；仍可主动用 CLI `--state-dir` 覆盖。省略 `--root` 时使用 `$XDG_DATA_HOME/agent-job-relay`，否则默认上述路径；个人 marketplace 请使用 HOME 内的安装目录。源码构建仍需 Rust。Claude Code 和 OpenCode 插件集成属于计划，尚未实现。官方插件平台是未来目标，当前不提交或发布。开发安装的 MCP 指向稳定安装目录中的绝对程序路径，即使宿主已有插件缓存，也须保留该安装。

如只配置独立 MCP，可将已安装启动器的 `setup plugin` 改为 `setup codex`。服务器指引同样包含当前终端绑定流程。不必在同一宿主/状态目录重复登记两种入口。

## 仅使用源码的开发快速开始

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

通知临时失败会在事件的 24 小时有效期内有界退避重试。会话变化或过期会等待显式恢复，
宿主能力不支持则立即停止。`delivery list` 显示 `failure_kind` 与 `next_attempt_at`；
`delivery retry` 保留投递 ID 和历史，不会重新执行任务。

## Desktop 重启后恢复通知

从同一聊天的终端执行已安装启动器：

```bash
"$AJR_INSTALL_ROOT/relay" session recover
"$AJR_INSTALL_ROOT/relay" session recover --apply
```

第一个命令刷新并探测当前绑定，预览可恢复任务，不迁移目标或发送聊天消息。
`--apply` 仅恢复相同逻辑聊天、调用者和宿主的通知，保留原投递 ID、有效期和尝试历史。
已接受或过期的事件不重发，正在投递的目标跳过，命令不会重新执行。
结果包含有界恢复审计及 `next_cursor`，后续页使用 `--after`。
共享 MCP 提供 `job_recover_session(session_key, apply)`，必须使用当前终端取得的新绑定。
此流程已有安装后 CLI/MCP 和假宿主测试；真实 Desktop 重启与继续执行仍待验证。
私有宿主身份仍来自可信调用者输入。

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
生成项指向**当前可执行文件**及状态目录；使用已安装的启动器即可保持位置稳定。setup 不安装系统服务，提交任务时会自动启动 Daemon。

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
  "args": ["--state-dir", "/absolute/private/state", "mcp", "--shared-host"]
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

安装的插件及生成的 MCP 配置使用 `mcp --shared-host`。Agent 从**当前聊天的终端工具**调用插件内启动器的 `session connect --brief`，再将刚得到的 `session_key` 传给 MCP。这由插件工作流程完成，无需用户复制 ID。

```json
{
  "spec": {
    "command": { "kind": "exec", "program": "cargo", "args": ["check"] },
    "cwd": "/absolute/workdir",
    "session": "<Agent 从当前终端绑定取得的新 key>"
  }
}
```

共享模式拒绝 `wake_me=true` 和 `job_connect_session`，因为服务器环境无法证明具体哪个聊天发起了调用。终端绑定会先探测真实当前会话能力，再提交需要唤醒的任务。若 MCP 不可用，技能可在同一终端用已安装启动器的 `start --wake`。缺少上下文时明确报错，不会选择其他聊天。

对于已明确隔离到单会话的 MCP 进程，手工运行不带 `--shared-host` 的 `mcp` 仍保留基于继承上下文的 `wake_me=true` 和 `job_connect_session`。wake_me 与 spec.session 不能同时提供，CLI 的 `--wake` 和 `--session` 也互斥。这是高级入口，不是安装插件后的默认流程。

自动登记租期为 24 小时，用 Socket 指纹标识宿主实例；每次请求唤醒都会刷新。
端点替换会产生新绑定，不会迁移已有任务。超过该租期的任务需要显式刷新或重新绑定。
宿主接受请求仍不代表 Agent 已恢复。CLI 快捷入口的首次启动、复用和空闲 Daemon 重启已在记录的 Desktop 版本与系统上通过用户确认的真实测试。安装后的共享 MCP 与两个终端绑定、单会话 MCP `wake_me=true` 均有自动化假宿主覆盖。用户已确认 0.1.2 更新后的 Desktop MCP 连通性通过；真实宿主上的共享 MCP 任务提交/唤醒和单会话 wake_me=true 仍待单独验证。

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

对于受管理的本地安装，先通过 Desktop 移除/禁用插件，预览并应用 `relay integration remove plugin`（独立 MCP 使用 codex/json）。检查活动任务，按需等待完成或取消，再执行 `relay daemon stop`。预览并应用 `relay uninstall --root <相同安装目录>` 只移除未改动的所属安装文件，保留任务数据及备份。宿主管理的插件缓存由宿主移除；安装目录中有用户修改或未知内容时会拒绝卸载。

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
当前没有 `data purge` 或系统安装包卸载器，不要删除未知目录或其他项目数据。插件升级与移除须默认保留历史、日志和用户配置。

在同一安装目录更新且没有覆盖状态配置时，会保留原有状态目录。先等待活动任务结束，再构建、停止该状态的 Daemon、重新安装并刷新宿主缓存。新构建的源码程序可直接按安装记录停止正确的 Daemon，不受无关 AJR_STATE_DIR 导出的影响：

```bash
cargo build --release --locked
./target/release/agent-job-relay daemon stop --install-root "$AJR_INSTALL_ROOT"
./target/release/agent-job-relay install --root "$AJR_INSTALL_ROOT" --apply
```

停止命令会等待 Daemon 释放锁。若已核实的早期 0.1.0 Daemon 不认识 `shutdown`，会通过 Linux pidfd 向原进程发送其已有 Ctrl-C 退出信号，并报告 `method=legacy_sigint`；不会向进程组发送信号或强制杀进程。install 本身不会停止 Daemon。更新前取消无关 AJR_STATE_DIR 导出；显式安装状态覆盖会主动选择另一份状态。

新版 Linux Daemon 即使程序文件被源码构建替换，也能从已加载程序启动 Runner；已有旧 Daemon 若原文件已删除，仍须显式空闲重启，不会自动重启或切换状态绕过。

## 常见问题与开发验证

- **Daemon unavailable：** 提交任务会自动启动相同状态目录的 Daemon；`doctor` 只检查，不会主动启动。
  自动启动失败时，用前台 `daemon` 查看诊断；没有安装系统服务。
- **看不到 MCP：** 更新本地程序包、刷新插件缓存并重启客户端。技能已加载不代表 MCP 已加载。
  修复后的 Codex manifest 已用 CLI 0.164.0 验证；`doctor --check-mcp` 是本地自检，`doctor --client ...` 检查所属配置，两者都不能证明 Desktop 连通。
- **Runner 要求重启 / 找不到程序：** `doctor` 会识别受影响的旧 Daemon。等待活动任务完成，再显式停止同一状态目录的 Daemon，用新版重新提交。保留失败任务；切换状态目录只是绕过问题，不是修复。
- **Agent 没有唤醒：** 插件使用当前聊天终端的 `session connect --brief` 绑定，然后向共享 MCP 传 spec.session。`start --wake` 是当前终端的 CLI 入口；wake_me=true 只用于专属单会话 MCP。
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

## Plugin 路线图

复用一个 Rust/MCP 核心，分别适配各宿主插件。Codex/Desktop 已有本地 Linux 开发程序包；Claude Code 和 OpenCode 集成属于**计划，尚未实现**。优先完善便捷安装、真实宿主工具加载、准确会话路由、升级和保留数据的移除。

未来优先使用各 Agent 官方支持的插件平台或安装机制。当前仍在开发阶段，**暂不向平台提交、不上传包、不公开发布**。独立系统安装包、软件仓库及其发布流水线已取消；插件内部仍可能需要按系统/架构提供原生运行时。实际支持范围及后续工作见 [Plugin 路线图](docs/TODO.md)。

MIT 许可。项目独立实现，只读研究 Vanth 的行为需求，没有复制实现，也没有创建或推送远程仓库。

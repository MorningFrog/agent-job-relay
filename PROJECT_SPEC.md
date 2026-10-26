# AgentJobRelay — 从零实现的工程任务规范

> 用途：将本文件放入新项目的根目录（建议命名 `PROJECT_SPEC.md`），交由 AI 在具有本机文件/终端权限的 Codex 或 Work 环境执行。 项目根目录：`/data/Projects/Agents/agent-job-relay` 仅供研究的参考目录：`/data/Projects/Agents/vanth-fork` 状态：工程实施规范，不代表以下功能已经完成。 **交付优先级：可靠性 + 用户体验。** 本阶段必须实现可靠的后台运行时、便捷的本地开发/测试入口、清晰的 `setup` / `doctor` / 集成移除、双语 README 和阶段性英文 Conventional Commits。**交付方向现已修订为 Plugin，独立系统安装包计划取消；当前不向任何官方平台发布。**

> **2026-10-26 用户修订（优先于原发行要求）：** 取消各系统独立安装包、包管理器及软件仓库发布计划，统一以 Agent Plugin 交付为目标，期望覆盖 ChatGPT Desktop/Codex、Claude Code、OpenCode，并尽可能通过各 Agent 官方支持的插件平台便捷安装。当前仍在开发阶段，暂不发布到官方平台，也不上传或配置发布凭证。保留一个 Rust/MCP 核心，各宿主分别实现插件入口和可验证的会话适配；Plugin 安装不等于已经支持自动唤醒。此修订替代旧的多格式安装包 TODO，详见新版 §16 和 docs/TODO.md。

## 0. 身份、总体任务与交付方式

你是本项目的首席 Rust 工程师、系统架构师和测试负责人。**请实际创建、实现、编译、测试并文档化项目，而不是只提出方案、生成伪代码或 TODO 清单。** 在当前环境允许的权限范围内持续推进到本规范定义的可验证阶段；不必为非关键技术选择反复向用户提问。遇到权限、宿主协议或缺少设备导致无法验证的事项，应提供可运行的替代测试、明确标记限制，绝不伪称已完成端到端验证。

创建一个名为 **AgentJobRelay** 的全新开源候选项目：一个跨平台、以 Agent 会话为一等实体的事件驱动后台作业运行时。它可以安全地启动独立于 MCP/Agent 客户端生命周期的长时间命令，持久化进度和结果，并在宿主允许时可靠地通知或唤醒**正确的** Agent 会话，让 Agent 在需要处理任务事件时继续工作，尽量避免在长任务期间持续轮询。

**设计目标是新架构，不是将 Vanth 翻译成 Rust。** 优先实现高可靠的核心闭环，不要照搬 Vanth 全部特性。**项目面向真实用户，而不只是开发者：从一开始就为简单、安全、可回滚的安装/卸载留下正确架构边界；当前优先保证源码构建、本地启动、MCP 客户端配置/移除和诊断体验。各 Agent 的 Plugin 交付为后续目标，当前只做本地开发及验证，不发布到官方平台。**

### 操作边界（必须遵守）

1. 仅在 `/data/Projects/Agents/agent-job-relay` 下创建和修改本项目的源码、测试、文档、构建配置与必要的临时测试文件。该目录不存在时可以创建；若已存在，先检查内容，不覆盖用户已有工作。
2. `/data/Projects/Agents/vanth-fork` 为**只读参考**，可查看代码、文档、测试、提交记录及许可证，但不得修改、清理或重建该仓库。它的真实内容由你在本机检查决定，不要臆造目录结构。
3. 不要修改用户现有的 `~/.codex/config.toml`、ChatGPT Desktop 配置、系统服务、全局 PATH、上游 Vanth 的数据目录或全局 AGENTS.md；确需集成安装时，先提供 dry-run、备份和明确的命令，未经授权不写入。
4. 不要运行 `sudo`、删除未知文件、清理其他项目目录、安装全局软件、公开发布、推送远程或创建线上仓库。允许在本项目目录进行常规 `cargo` 构建、依赖解析、测试和文档生成。**允许在项目仓库中执行 `git init`（仅限尚未初始化的目录）、`git add` 与本地 `git commit`，并要求按 Conventional Commits 在每个阶段提交；不得 push、发布 release 或改写已有历史。**
5. 尊重适用的本地/上级 `AGENTS.md` 与相关 Skill。文件使用 UTF-8；不采用隐藏的降级或忽略失败来制造“测试通过”。
6. 不要读取/输出与本任务无关的私人文件或秘密；执行测试时使用隔离的数据根目录、随机或临时的 IPC 地址，不接入现有 Vanth 生产数据。
7. 不要因为某个重要集成不可用就以不安全或语义不同的实现静默替换；要做能力检测、给出明确错误或受控降级说明。

## 1. 开始时先进行实证调研（时间盒：避免陷入无限分析）

先阅读本机 Vanth 副本中与以下主题相关的**少量关键文件**：README、LICENSE、后台 Daemon/Runner、MCP 入口、持久化 Job/Event/Delivery、Codex/ChatGPT Desktop 唤醒、相关回归测试。尤其检查该副本是否包含曾解决 Linux Unix Socket、`callerSource`、MCP 进程组误杀 Daemon、`job_stop` 参数等问题的修复；不要默认其分支或版本与上游相同。

输出简明的 `docs/reference-audit.md`，至少记录：
- 从 Vanth 借鉴的**行为需求和负面经验**、可以复用的测试思想；
- 不应继承的复杂性（通用 artifact 系统、SSH Beta、过度宽泛的调度系统等）；
- 要避免的四类耦合：作业运行、MCP 客户端、Agent 会话和消息投递的生命周期耦合；
- Vanth 和新项目之间的实质性架构差异；
- 代码来源/许可声明（如使用任何可版权保护的实现片段则按许可证保留声明；首选独立实现，不整体复制）。

验证**当前实际可安装**的 Rust crate API/版本，而不是依赖过时的示例：`tokio`、官方 `rmcp`、`rusqlite`、`clap`、`serde`、`tracing`、`interprocess`（是否必要）、`thiserror`、`windows-sys`/`rustix` 等。特别注意 RMCP SDK 3.x 可能有破坏性变更。读取官方文档或仓库 README，以构建结果为准。**不要固定未经验证的 crate 版本号。**

不要花费过多时间做广泛竞品研究；调研后立即形成设计记录并进入实现。

## 2. 核心产品定义与非目标

用户使用情景：Codex CLI 或 ChatGPT Desktop Work/Codex Agent 启动一个 20 分钟构建、训练或数据处理任务。任务在原 MCP Server 结束、Agent 会话休眠或 Desktop 窗口关闭时仍可按明确约定继续运行。任务结束时结果和事件不会因 Daemon 暂时断线而无声丢失。当 Host 存在受支持的恢复能力时，系统将关联事件投递到发起任务的正确会话，而不需要 Agent 每隔数秒轮询；当 Host 不支持自动恢复时，系统仍提供 CLI/标准 MCP 查询和清晰告警，不声称已经自动恢复。

核心功能范围：本地命令执行、独立进程监督、Job/Event/Session/Delivery 持久化、本地 IPC、MCP、CLI、能力感知的 Host Adapter、可诊断的任务恢复。

明确**非 v0.1 目标**：云端服务、远程 SSH Worker、Web UI、完整 artifact registry、复杂 Cron 调度、多人 RBAC、TLS 公网接口、分布式队列、任意历史 Desktop 会话的魔法唤醒。可以预留接口，但不得为未实现功能增加大量空壳。

软件应为单用户本地可信环境设计，实际命令仍以该用户的 OS 权限执行；这不是代码执行沙箱。不要对用户不可信内容做未经授权的自动执行。

## 3. 推荐技术栈（如需修改须在 ADR 解释）

- 单一核心语言：Rust stable / Edition 2024。
- Tokio：异步 I/O、进程和任务协调。
- `rmcp`：官方 Rust MCP SDK，优先 stdio；如客户端协商支持，可在后续阶段支持标准 MCP Tasks，**不得把 Tasks 误认为自动恢复 Agent 对话**。
- `rusqlite` + SQLite：显式 SQL、schema migration、WAL、busy timeout、事务；使用单写者 DB Actor 或受控阻塞线程，避免直接在 Tokio runtime 上执行阻塞 SQL；不能跨 `.await` 长时间持有数据库锁。
- `clap`：CLI；`serde` / `serde_json`：消息协议；`thiserror` / `anyhow`：分层错误；`tracing`：结构化日志。
- Unix Domain Socket 与 Windows Named Pipe：本地 IPC；优先可维护的小型适配封装，必要时使用 `interprocess`；IPC 身份与权限必须按平台验证。
- Linux/macOS 进程组与 session、Windows Job Objects：将 OS 特殊性限制在 `platform/` 内。其他 OS 作为后续兼容目标。
- 测试：Rust 内置测试、`assert_cmd`/`tempfile`/`proptest` 等按需使用；GitHub Actions 多平台检查矩阵；插件发布自动化待后续授权，本轮不实现。
- 可选 TUI：`ratatui`，严格排在可靠唤醒和恢复之后。
- 发行与安装：开发阶段保留 cargo build/run/install/uninstall 及本地插件安装入口。用户最终通过 Agent Plugin 使用同一 Rust/MCP 核心，期望覆盖 ChatGPT Desktop/Codex、Claude Code、OpenCode；逐一适配其官方安装机制。当前不发布，独立系统安装包及软件仓库计划已取消，详见 §16。

坚持模块化单体，首先采用**一个 Cargo Package、一个可执行文件、内部清晰模块**。不要为了形式主义立刻拆成多个发布 crate；若后续需要才拆分。可执行文件内部提供 `daemon`、`mcp`、`runner` 等运行模式。

## 4. 建议目录（可以有合理调整，不能过度工程化）

```
agent-job-relay/
  Cargo.toml
  Cargo.lock
  AGENTS.md
  PROJECT_SPEC.md
  README.md
  README.zh-CN.md
  .gitignore
  LICENSE
  src/
    main.rs
    core/{job,run,event,session,delivery}.rs
    daemon/{manager,recovery,dispatcher}.rs
    runner/{supervisor,process,output}.rs
    storage/{mod,sqlite,migrations}.rs
    ipc/{mod,protocol}.rs
    adapters/{mcp,codex_cli,chatgpt_desktop,opencode}.rs
    platform/{mod,unix,windows}.rs
    cli/mod.rs
    config.rs
  tests/{cli,daemon,runner,persistence,delivery,mcp}/
  docs/{architecture,reference-audit,protocol,security,limitations,testing,installation,release,TODO}.md
  .github/workflows/ci.yml
```

不存在的平台适配器不要制造全部实现已完成的错觉；可用 feature gate 或显式 Unsupported 状态。模块文件名和代码结构由实际实现决定，不要求为目录示意图创建空文件。

## 5. 必须独立建模的领域实体

至少包括：
- `Job`：作业的用户意图、命令、工作目录、创建者会话、创建时间、当前状态、唯一 Job ID；
- `Run`：某 Job 的一次具体执行，Run ID、attempt、进程实例身份、退出代码/信号、状态及时间戳；
- `Event`：稳定 Event ID、Job ID、Run ID、事件类型、单调序号/排序规则、时间和受限大小的 payload；
- `AgentSession`：agent_kind、逻辑 session_id、instance_id、host/runtime 身份、连接状态、能力、可验证的生命周期与注册租期；
- `WakeTarget`：对特定 Job/Event 的目标 Agent 会话、目标身份和触发事件；
- `Delivery`：事件、目标、投递尝试、状态、租约、重试和可诊断的错误。

Job、Run、Session、Delivery 必须独立拥有生命周期。**Job 完成 != 通知送达 != Host 接收 != Agent 真实恢复执行。** 不能把它们压缩成一个布尔状态。

命令定义应优先为强类型 `Exec { program, args, cwd, env... }`；如需管道/重定向可显式用 `Shell { interpreter, script }`。Shell 必须是显式用户选择，不得从字符串模糊识别并自动拼接。工作目录必须明确解析，输出路径、环境变量也应可审计。

任务 ID 不应依赖 OS PID；重启/恢复时不只凭 PID 判断进程是否还活着，必须记录并验证创建时间或其他 OS 实例身份（可行时）。禁止将复用 PID 当成同一任务。

## 6. 核心流程与持久化正确性

### 6.1 启动

1. 验证命令、路径、环境及配额，生成稳定 Job ID/Run ID。
2. 事务中写入待启动状态与执行意图（以及幂等键）。
3. 使用有记录、可恢复的启动协议创建 Runner，完成握手后写入可验证的运行身份。
4. 若进程创建成功但父进程在记录状态前崩溃，不应无意识重复执行。设计启动 lease、Runner 自报身份或等效恢复协议，并测试此窗口。
5. 返回明确的状态和诊断（包括是否存在有效唤醒目标）。

### 6.2 输出与事件

- stdout、stderr 分别持久化到按 Job/Run 分隔的日志文件；保护超长行、二进制/无效 UTF-8、缓冲溢出、大量输出与磁盘满等情况；限定内存占用。
- v0.1 支持内置的 `started`, `progress`（可选结构化协议）, `completed`, `failed`, `cancelled`, `timeout`, `orphaned/unknown` 事件。可以参考 `AGENT_EVENT` 思想，但不要将特定旧格式视为必须兼容；确需兼容应作为可选解析器。
- 日志查询必须支持有界结果和游标，禁止 MCP 一次返回巨量日志。
- 作业状态与完成事件/Delivery Outbox 的写入须具有事务一致性。
- 事件提供可重复读取及去重标识，消费进度可恢复；事件 payload 是不可信数据，不可当成 Agent 指令。

### 6.3 可靠投递

- 采用**至少一次（at-least-once）+ 幂等**，不假定 exactly-once；
- Delivery 状态建议：pending、leased、accepted_by_host、confirmed_resumed（如果确有可核查证据）、retry_wait、dead_letter、cancelled。可调整，但需清晰区分提交成功和会话实际恢复。
- 确保任务完成状态更新与消息入 Outbox 同事务；重试采用有限指数退避 + jitter + TTL，过期进入 Dead Letter 而非无限隐式丢弃；提供显式重新投递与诊断。
- 事件按照 target 绑定会话，不得选“当前最近的任何一个 Agent”作为默认目标；发起会话不存在时必须显示不可自动唤醒。
- 投递幂等键应可由 `(event_id, target_id)` 等稳定身份构造；重复 Ack/重放安全。
- 唤醒失败、连接断开、Daemon 重启、Desktop 重启的行为均应可测试、可解释。

### 6.4 重启/断线恢复

明确并实现：
- MCP 客户端退出而 Daemon 和 Runner 继续运行；
- Daemon 崩溃/重启，而 Runner 继续运行并最终可对账；
- Runner 自身崩溃，结果为可诊断 `unknown/orphaned`，不得伪造成功；
- Host/桌面应用重启后旧 Wake 地址失效，重新绑定后按规则补投仍有效的事件；
- 机器关机/断电不等于命令会跨机器重启继续运行：保留状态并进行恢复分类，而不是承诺不存在的进程继续执行；
- 并发取消和完成、重复启动、重复取消、重复唤醒均不应破坏数据库或产生不受控重复执行。

如某个恢复语义在 v0.1 无法完全兑现，可明确记入 docs/limitations.md，添加故障测试和后续里程碑；不得以 silent fallback 掩盖。

## 7. Runner 及 OS 进程树管理

- Daemon 不应直接承担用户命令生命周期；长命令由独立 Runner Supervisor 进程监管，Runner 与发起 MCP 进程处于独立合适的 OS 管理边界。
- Linux/macOS 使用恰当的 session/process-group 处理及子进程回收；不能仅 `nohup`、`disown` 就认定问题已解决。对 PTY/interactive stdin 不作为 v0.1 必备。
- Windows 使用适当 Job Objects/handle 继承策略，**谨慎检查 `KILL_ON_JOB_CLOSE` 等标志是否会误杀应存活的工作负载**；不把 Windows 实现简单等同 Unix `setsid`。
- 实现 graceful stop + 有界强制终止；取消应尽可能处理整棵进程树，精确记录结果和原因。
- 超时机制、最大并发数和资源配额应有基本实现或明确限制；资源限制不应被描述为安全沙箱。
- Daemon、MCP 与 Runner 子进程之间避免不必要的共享 process group，尤其不能因为 stdio MCP cleanup 而停止整个后台作业。
- 防止僵尸进程、错误的文件描述符继承、标准输出 deadlock 和 Runner 永不退出；平台兼容用真实操作系统集成测试验证。

## 8. Agent/MCP 接口

MCP server 首先用 stdio，且保持 stdout 是纯 MCP 通道，诊断输出只写 stderr/日志。建议核心工具：

- `job_start`：启动，返回 job_id/run_id/status/wake capability；
- `job_status`：稳定状态与终态摘要；
- `job_list`：有界查询与分页；
- `job_logs`/`job_tail`：有界日志+游标；
- `job_events`：有界事件+游标；
- `job_stop`：幂等取消；
- `job_wait`：**有界**事件等待，不能无限阻塞或要求 Agent 无休止轮询；
- `job_doctor`：daemon/runner/store/relay/wake 健康诊断；
- `job_register_session`/`job_add_wake_target`：若 MCP Host 无法可靠自动提供会话身份，应以显式注册方式完成，不要编造会话 ID。

Tool 描述应引导 Agent：短任务可以等待，长任务在有可靠 Wake 能力时 `start` 后结束当前回合等待唤醒；无 Wake 能力时诚实提示，禁止循环频繁轮询。MCP Tasks/订阅功能仅在 SDK 和 Host 实际协商支持后启用，不应破坏普通工具兼容。

CLI 第一版推荐：

```
agent-job-relay daemon
agent-job-relay mcp
agent-job-relay start -- <program> <arg...>
agent-job-relay start --shell <script>
agent-job-relay status <job-id>
agent-job-relay list
agent-job-relay logs <job-id> [--follow]
agent-job-relay stop <job-id>
agent-job-relay doctor
agent-job-relay setup [client] [--dry-run]
agent-job-relay integration remove <client> [--dry-run]
```

最终根据 clap 的语义给出可运行的真实示例，保留 `--` 参数边界，避免 Unix Shell/Windows PowerShell 的引号陷阱。CLI 名称暂用 `agent-job-relay`，不强制增加 `ajr` 别名。**`setup` 和 `integration remove` 要做到可预览、幂等、可诊断，并仅改变本项目管理的配置项；确切语法可以合理调整，但必须在两种 README 中一致。**

## 9. Agent 唤醒适配：这是主要差异化功能

请定义面向能力的适配器接口，至少表达：
- can_notify / can_resume / can_confirm_resume；
- requires_live_host / supports_reconnect；
- register、probe、deliver、ack/confirmation、invalidate、refresh。

### Codex CLI

先研究当前公开支持的 CLI/app-server/会话恢复机制，区分：终端中新建命令、恢复一个已结束线程、向运行中的会话提交后续消息。这三者不能混为一谈。不要未经证明声称 `codex resume` 一定能无用户交互地恢复任何运行中的会话。允许在 v0.1 先实现可可靠测试的通知 adapter，实测通过后才标注为真正的 auto-resume。

### ChatGPT Desktop Work / Codex

上游 Vanth 的桌面桥接依赖未经公开稳定承诺的 Host pipe/Unix socket + JSON-RPC（相关字段如 `callerSource`），存在单目标、宿主重启失效、Host Ack 与 Agent 真正执行不同步等限制。新项目必须**将此机制隔离为显式 experimental adapter**：
- Linux Unix Socket、Windows Named Pipe 采用各自正确连接方法；其他平台按实测支持；
- endpoint/path、会话身份、session/instance 生命周期绑定，能够检测过期；
- 多会话不得靠单个全局覆盖式 `desktop.json` 实现；需要区分不同 Host 实例/对话，但能否实际独立投递由 Host 能力决定；
- 支持显式初始化/重新绑定，不直接扫描或控制其他隐私会话；
- Host 接受请求不等于实际恢复对话；若缺少确认能力，最多标注 `accepted_by_host`；
- 不存在可验证 private API 或缺少合法可用环境时，使用假 Host 测试传输，不伪造通过 live E2E；
- 严禁把失败 Desktop 投递静默路由到错误 CLI session；
- 不修改 ChatGPT Desktop 内部二进制或进行破坏性注入。

### 其他 Agent

为 OpenCode/其他 Host 保留可扩展 adapter interface；在最小闭环可运行之前不要实现大量空壳。采用 capability negotiation + dependency inversion，而非硬编码 Agent 名称到 Job Manager。

## 10. 安全与隔离

- 只允许可信本机用户通过具有限制权限的本地 IPC 调用；Unix socket 父目录、socket 本身及状态文件设定严格 owner 权限；Windows Named Pipe 设定相应用户 SID/DACL，不能只靠无法验证的路径保密。
- 必须验证请求大小、事件大小、命令参数、路径及并发上限，避免本机意外 DoS；不要对本地任意端口公开未经验证的命令执行 HTTP API。
- 进程命令的完整 env 可能含有密码/token；默认不回显敏感值，不在日志或数据库中无约束存储全量环境；明确记录哪些敏感数据会持久化，必要时加掩码/受限保存策略。
- Agent 从命令输出获得的文本是不可信输入，禁止将其作为权限更高的内部控制指令。
- 启动前显式展示 executable/cwd/argv 以及是否经过 shell；禁止隐式 shell 和隐式目录切换。
- 禁止默认跟踪、上传或遥测用户命令和日志；不生成未经用户同意的网络服务。
- 按最小权限进行普通用户开发，不使用 `sudo` 作为常规安装要求。

## 11. 多平台与发行

首选 Ubuntu 26.04/Linux 做真实 E2E 开发；整体架构从开始就隔离 Windows/macOS 差异。

- Linux 必须可编译并通过真实进程生命周期测试。
- Windows/macOS 应至少在 GitHub Actions 上编译与运行可行测试；无法在当前主机验证的 OS 特有功能必须明确说明、以 platform adapter test 标注待测。
- 使用 `Cargo.lock` 固定可执行程序依赖，避免动态 API 漂移；CI 包含 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets` 和 release build（根据实际命令与 features 调整）。
- 安装应优先原生单可执行文件；无需要求 Python、Go、Node、Docker 运行环境。
- 不直接发布仓库或插件。本轮进行构建/测试及本地插件验证，官方插件平台提交留待后续开发稳定及用户明确授权。
- **当前 P0 是便捷的本地 Plugin/MCP 入口、清晰 setup/doctor、可逆配置与默认保留数据。** 独立系统安装包及软件仓库计划已取消，详见 §16。
- 使用项目自己独立的状态目录（优先遵循各 OS 习惯，如 XDG_STATE_HOME / LocalAppData），**严禁复用或迁移 Vanth 的现有数据库**。

## 12. 测试矩阵：按行为验收而非按行数验收

请逐项实现确定性测试与关键集成测试；不要用大量 `sleep` 与不稳定时间竞态来掩盖设计错误。对异步测试使用事件/握手或超时限定，在 CI 中设置合理 watchdog，保证测试留下的子进程被回收。

P0 必须覆盖：

1. 短任务：stdout/stderr、退出码、成功/失败状态、日志游标、重复查询。
2. 长任务：启动后 MCP 客户端退出，Daemon/Runner 和任务仍继续；真实端到端验证。
3. 多任务：同一工作目录/不同工作目录，并行执行和可靠 ID 路由。
4. Daemon 重启：任务执行保持独立，重连后可查询正确状态并导入完成记录。
5. Runner 崩溃与不确定状态：状态不伪造成功，诊断准确。
6. 启动竞态：重复 idempotency key 不重复执行；中断启动后不无意重放。
7. 完成事件/Outbox 一致性：故障注入与事务边界检查。
8. 投递重复/失败/断网/Daemon 重启：重试与去重、dead letter、重新投递。
9. 取消及超时：完整的子进程树处理、不发生误杀其他不相关进程。
10. 会话路由：A 会话的完成事件绝不能发送给 B 会话；旧 Host 实例重连后如何重新绑定。
11. Desktop 假 Host：模拟传输、字段校验、Host Ack/未恢复的差异、Host 重启/连接失效、不能静默回退。
12. 权限与安全：IPC 文件权限、无效/畸形消息、巨大日志、敏感信息泄露检查。
13. 宿主机重启和 Runner 消失时的状态恢复策略（无法真实测试断电时，用故障注入说明可验证范围）。
14. MCP：协议握手、工具发现、核心工具实际执行，与普通 MCP 客户端兼容。
15. 当前用户体验：从干净 HOME/测试数据目录中验证本地构建后的首次启动、`setup --dry-run`、`doctor`，以及重复 setup 不覆盖其他 MCP 项；**不要求本阶段实际构建 `.deb`/`.rpm`/`.AppImage`/`.exe`/`.dmg`/Flatpak/PyPI Wheel**。
16. 当前移除体验：测试 `integration remove` 的 dry-run、可逆配置变更、保留用户任务数据；记录本地 `cargo uninstall` / 删除自行安装二进制的注意事项。Plugin 移除须保留数据；不再规划独立系统包卸载器或包管理器测试。
17. 文档验证：两个 README 的**当前可执行**本地安装/运行、`setup`/`doctor`/集成移除命令必须匹配真实 CLI；各宿主 Plugin 的实际支持和未实现项应明确标注；验证两个 README 开头都有英中双语简短介绍。

真实 ChatGPT Desktop/真实 Codex Agent E2E 只有在测试环境明确可用时再运行；提供手动 E2E 文档、通过的版本号/客户端及观察记录。不能把 fake Host 的测试当成真实 Desktop 集成验证。

## 13. 分阶段实施与 Gate（不要只停留在计划）

### Stage 0 — 参考审查与设计冻结
产出：`docs/reference-audit.md`、`docs/architecture.md`、关键 ADR、领域状态机、事件/IPC 协议草图及不确定点；立即准备 Cargo 项目，完成 `cargo check`。在本阶段可以用 Markdown/Mermaid 图解释关键生命周期，但不要让设计文档无限膨胀。

### Stage 1 — Core v0.1：可以执行并恢复的最小闭环
产出：项目初始化、clap CLI、Daemon/IPC、SQLite migrations、Job/Run/Event、独立 Runner、日志、stop/status/list、基本可靠启动/对账、真实 Linux 生命周期测试。命令行必须有可理解的 `--help`、错误消息与退出码；开发阶段允许用户通过 `cargo run` 或已构建的可执行文件启动，不要求立即提供免编译的软件安装包。**先让这部分可靠运作，而不是同时开发所有 Agent 适配器。**

### Stage 2 — MCP + Durable Delivery
产出：rmcp stdio server、工具层、Session/Target/Delivery 数据模型、事务性 Outbox、去重、退避、TTL/dead-letter、假 Host Adapter、完善诊断与测试；同时实现可预览的客户端 `setup`/`integration remove` 最小可用闭环，避免要求用户手改 MCP JSON。

### Stage 3 — Agent 集成与多会话
产出：可验证的 Codex CLI 集成；对 ChatGPT Desktop 构建严格隔离的实验适配器，兼容 Unix Socket/Windows Pipe（实测允许的平台），多会话注册逻辑，重新绑定；实测无法保证的能力必须作为限制。

### Stage 4 — 质量门禁及文档
产出：CI 矩阵、稳定性/故障注入测试、**英文 `README.md` + 中文 `README.zh-CN.md`（两者开头均含英中文双语简介）**、安装/卸载与 MCP 一键配置文档、CLI 示例、恢复说明、`docs/security.md`、`docs/limitations.md`、`docs/testing.md`、版本/变更记录，以及**当前实际可运行的**源码构建/本地安装/集成移除说明和基础 smoke test。在 `docs/TODO.md` 维护各宿主 Plugin 交付和未来官方平台计划，**当前不得发布，独立系统安装包计划已取消**。实际运行 fmt/clippy/test/release-build 并记录结果。

如果自动化工作会话限制导致无法一次执行全部 Stage，应将所有已实现内容保存在仓库、完成相应测试并将后续任务写入 `docs/IMPLEMENTATION_STATUS.md`；下次从这个文档与 Git 历史恢复，不能只输出大段“以后再做”。

### 通过 Gate 的严格定义
- 功能已编译且对应自动化测试通过，才可标记 `implemented and tested`。
- 使用假 Host 验证仅标记 `simulated`。
- 真实 Desktop 唤醒只有现场实测、回到**同一指定会话**并产生后续 Agent 执行后才能标记 `live E2E verified`。
- 缺少平台测试只能标记 `not verified`；不要把 Linux 测试通过写成 Windows/macOS 全部验证。
- 不得删除失败测试、降低 lint 严格程度或改坏功能换取绿色 CI。
- 如为阶段性 MVP，清晰写出未完成功能，避免 README 预支未来能力。
- **当前本地运行、Plugin/MCP、setup/doctor、集成移除必须有测试或手动验证**。其他宿主和未来官方平台提交须单独标注未实现/未验证，不允许写成已支持；系统安装包已取消。
- Stage 0 至 Stage 4 均需要有**阶段性本地 Git 提交**，代码变更按有意义的功能切分，使用英文 Conventional Commit 消息；不得把所有改动压成一个最终提交，也不得在测试失败时标称完成。

## 14. 文档、维护规范与 AGENTS.md

生成简短、实际有用的项目 `AGENTS.md`，写入可持续约束：
- 先读 `PROJECT_SPEC.md`、`docs/architecture.md`、`docs/IMPLEMENTATION_STATUS.md`；
- 修改涉及任务状态机/投递协议必须补对应测试；
- 永远区分 accepted_by_host 与 confirmed_resumed；
- 进程树/数据库/IPC 安全为高风险修改，严禁绕过测试；
- 跨平台代码以 `platform/` 隔离；
- 使用 UTF-8；测试隔离、不要碰上游 Vanth 状态；
- 不要因为未实现功能而生成虚假文档、假接口或声称成功；
- 尽量以公开官方接口替代不稳定私有 Host API；任何 private API 变更需写出兼容风险。
- 优先提升当前用户体验：可运行的本地构建入口、清晰 setup/doctor、可逆 MCP 配置；Plugin 便捷安装和安全移除是交付目标，独立系统安装包已取消；当前不发布到官方平台。
- README 英文版 `README.md` 与中文版 `README.zh-CN.md` 均应有英中文简短介绍，并及时同步安装/卸载等用户可见内容。
- 使用 Conventional Commits 规范，Git 提交标题及正文均使用英文；阶段性提交、不得 push 或重写历史。

同步维护 `docs/IMPLEMENTATION_STATUS.md`：当前版本、完成的 Stage、通过/失败的测试、实际已证实的宿主功能、已知缺陷、下一步优先事项和最后的验证命令。文档应与代码及运行结果一致。

**README 国际化是必需项。** `README.md` 为默认英文完整版，`README.zh-CN.md` 为内容一致的中文完整版。两个文件的最前部（标题之后、目录及长篇说明之前）都必须分别提供**英文一小段介绍 + 中文一小段介绍**，不能只提供单语正文加一个语言链接；同时提供互相跳转的 `English | 简体中文` 链接。两者都包含**当前可执行的**开发者安装/运行、集成移除、一分钟 Quick Start、MCP 配置、CLI 示例、数据保留与清理警告、常见问题与故障排查；各宿主 Plugin 的未实现能力和未来发布必须明确标注，不能冒充可执行命令；新功能或 CLI 变更需同步更新双语文件。不得宣称尚未接入或发布的插件平台可用。

## 15. 实现风格与架构约束

- 优先清晰、可审查、易测试的代码，而非引入大量宏、泛型抽象、框架或构建层。
- 独立抽象 OS 平台能力，而不是将各系统特殊行为散落业务层。
- 将不可逆副作用与数据库事务边界清楚分离，显式记录中间状态/恢复策略。
- 所有非短暂状态都要有明确的所有者和恢复语义；消除 fire-and-forget 且不可诊断的关键任务。
- 日志/事件/指标不得无限增长；设计合理的大小限制、分页及可选 retention（清理需安全）。
- API 约束、错误返回、schema version 与升级/回滚策略应明确，不能盲目兼容 Vanth 的旧格式。
- 在相同功能下，优先降低 Agent Token 消耗：给出精炼状态与错误、有限日志切片，避免为了“动态进度”而引导主动高频轮询。
- 不能将 `Agent wake` 的成功依赖于 Agent 自身保持运行；对不支持异步恢复的 Host 提供严格的 capability 说明。

## 16. 用户体验与 Plugin 交付（2026-10-26 修订）

### 16.1 统一交付方向

- 以 Agent Plugin 为用户入口，用户安装/配置一次后即可用自然语言调用后台工具，不应在每次提示词中填写二进制路径、状态目录或会话 ID。
- 期望覆盖 ChatGPT Desktop/Codex、Claude Code、OpenCode。复用单一 Rust/MCP 核心，每个宿主按其官方机制提供插件描述、技能/工具、安装和会话适配。
- 当前只允许本地开发、源码构建和隔离验证，不发布到官方平台、不推送仓库、不上传包、不配置线上凭证。未来以各 Agent 官方支持的插件平台/安装机制为优先目标；具体上架与审核须另行核定、另行授权。
- 独立系统安装包、包管理器、软件仓库及其自动发布计划已取消，不应继续登记为待实现任务。插件内部按 OS/架构提供原生运行时不等同恢复系统安装包计划。

### 16.2 当前必须实现的基础体验

- 保留可执行的源码构建、CLI、setup/doctor、安全移除和稳定的开发安装入口；开发阶段允许构建一次后本地安装插件。
- 安装、状态数据、Host 会话和 MCP 连接保持独立。升级时未显式更改状态目录应保留原有配置及历史，插件缓存更新不能使运行中的新版本 Daemon 失去启动 Runner 的能力。
- 配置默认预览，--apply 仅改动所属项目条目并备份。仅在隔离 HOME/配置目录自动验证，不擅自修改真实配置、服务、PATH 或其他项目。
- doctor 分别报告配置匹配、运行时健康、自身 MCP 协议检查和真实 Host 工具连通性，不把本地自检当成宿主已安装工具。

### 16.3 各宿主适配及安全移除

- 逐一实测宿主插件加载器，不能假定一个 portable manifest 在所有宿主通用；优先各平台正式支持的插件格式与本地开发方式。
- 每项任务绑定真实发起会话。共享 MCP 不得使用进程继承的旧会话身份猜测调用者。Claude Code/OpenCode 的身份、通知和继续执行能力必须独立实现并验证，不沿用 Codex 私有端点伪称跨宿主支持。
- 插件已显示、技能可用、MCP 工具实际加载、任务执行、Host 接收和 Agent 真实继续是不同验收项；降级到 CLI 必须记录实际入口，不能称为完整 MCP 验收通过。
- 插件移除、marketplace 条目移除、缓存清理和用户数据清除是不同操作。正常移除默认保留任务历史、日志及个人数据，拒绝覆盖/删除第三方或用户修改的内容。

### 16.4 验收与后续计划

- 自动验证安装、升级、配置幂等、跨目录启动、状态一致性、其他插件保留和安全移除；对真实暴露的问题补回归测试。
- 人工验收优先使用一次安装后的真实任务，用户提示词不含工具内部路径；不要求重复已通过的 CLI 测试或执行不相关的人为故障矩阵。
- 维护 docs/TODO.md 中的 Plugin 交付路线图、各宿主的实际支持状态及未来官方平台工作；未实现和未验证的宿主不得写为支持。
- 官方平台发布留待开发稳定且用户另行授权之后。当前不创建发布流水线或执行线上操作。

## 17. 双语 README、Git 规范与阶段提交（必需）

### 17.1 英文默认 README + 中文完整 README

- 必须提供 `README.md`（英文为主）和 `README.zh-CN.md`（简体中文为主）；不是只翻译标题，也不是英文版完整而中文版只有一段介绍。
- **两个 README 开头都要出现英中文双语的简短项目介绍**，且随后提供双向导航 `English | 简体中文`。即使用户直接打开中文版，也能在首页顶部看到英文简介；直接打开英文版，也能在首页顶部看到中文简介。
- 推荐格式（两个文件均适用）：

  ```markdown
  # AgentJobRelay

  Run long-running jobs in the background and automatically resume AI agents when needed.

  在后台执行耗时任务，并在需要时自动唤醒 AI Agent 继续工作。

  **Language / 语言：** [English](README.md) | [简体中文](README.zh-CN.md)
  ```

- README 后续全文按各自语言书写，且两种语言的功能表、真实可用平台、安装/卸载命令、CLI/MCP 选项、已知限制与安全提醒**语义保持一致**。命令、路径、参数和示例必须能与实际实现相匹配；如功能未实现，标注计划中，不能写成已经交付。
- 两种 README 都必须明确区分：Job 运行完成、通知被 Host 接收，以及 Agent 真实恢复工作；明确显示 Desktop 实验性 API 和不可保证的场景。
- 可加入 FAQ/截图/架构图，但优先可操作的步骤和快速故障排查，不让初次安装者先读架构设计。

### 17.2 Conventional Commits（英文）

- 若项目目录尚未有 Git 仓库，先在**目标目录中**初始化 Git 并设置适当 `.gitignore`；如已有仓库，先检查 `git status` 和历史，绝不覆盖未提交的用户修改。
- 使用 **Conventional Commits** 的统一格式：`<type>(<optional-scope>): <English imperative summary>`。常用 type：`feat`, `fix`, `refactor`, `test`, `docs`, `build`, `ci`, `chore`, `perf`；破坏性变更按规范标记 `!` 和/或 `BREAKING CHANGE:`。
- **所有 Git commit 的标题与正文都必须是英文**，标题清楚描述实际修改，不能使用 `update`, `misc`, `changes` 等不明确的消息。代码注释和文档语言按用途决定，Git 提交不使用中文。
- **必须在实施过程中阶段性提交**：项目初始化/架构、可运行的 Job/Runner、持久化与恢复、MCP/投递、Agent 集成、当前 setup/集成移除、双语文档、基础 CI、发行 TODO 等达到可验证的增量时及时提交；至少每个已完成 Stage 留下独立可识别的本地提交。不要直到最后才一次性 `git add . && git commit`。
- 倾向于每次提交一个相关且可审查的变更，提交前检查 `git status`、`git diff --check`，对受影响组件运行相应的格式化、编译与测试；不要明知无法构建仍声明稳定完成。如需要暂存实验，放在 feature 分支并注明，不要污染稳定主线。
- 使用 `git add` 精确选择路径，禁止将 `.env`、访问令牌、SSH key、真实日志、数据库、测试数据、编译产物、用户隐私文件加入版本库；适当地忽略 `target/`、本地数据目录和临时文件，但应提交 `Cargo.lock`（可执行应用）。
- 只允许**本地**提交。未经用户明确许可禁止 `git push`、创建远程仓库、发布 tag/release、`git reset --hard`、`git clean -fd`、重写历史或执行破坏性 Git 操作。若身份缺失，先检查已有 repo-local Git 配置；不能擅自改用户全局配置或捏造身份，可报告此阻碍，保留可审查的变更。
- 合理的示例提交序列（按真实进度调整，不能预先造假提交）：

  ```text
  chore: initialize Rust project and development tooling
  docs(architecture): define job and delivery state machines
  feat(runner): supervise detached commands and capture output
  feat(storage): persist jobs and completion events atomically
  feat(mcp): expose background job tools over stdio
  feat(delivery): route durable wake events to registered sessions
  feat(cli): add idempotent agent setup and integration removal
  test(recovery): cover daemon restart and duplicate deliveries
  docs(roadmap): define host plugin delivery targets
  docs(readme): add bilingual onboarding and uninstall instructions
  ```

### 17.3 交付时的 Git 与文档验收

- 验证 `README.md`、`README.zh-CN.md` 同时存在，并且两者开头均有真实英文简介与中文简介、语言切换链接。
- 验证当前可执行的源码构建/本地运行、首次 setup、诊断、CLI 使用及移除集成在双语文档中与当前能力一致；其他宿主插件/可选数据清理若尚未实现，必须明确标注；系统安装包计划已取消。
- 输出 `git log --oneline` 的主要阶段提交及最终 `git status`；明确是否存在尚未提交的开发内容、跳过的验收或未验证的平台。

## 18. 交付输出的格式（完成时）

请给出**真实执行的结果**，不是计划书：

1. 项目根目录和生成的核心文件；
2. 重要架构决策摘要及相对 Vanth 的差异；
3. 已实现功能 / 模拟测试功能 / 尚未实现功能三类清单；
4. 每个测试命令、实际通过/失败数量及平台；
5. 真实 Agent 唤醒是否已完成 E2E，若没有，准确指出阻碍；
6. 在当前环境如何启动 Daemon、CLI、MCP，以及可验证的本地构建/安装方式；
7. `docs/IMPLEMENTATION_STATUS.md` 的后续交接信息；
8. 安全限制、已知风险与优先下一步；
9. **当前真实可用**的本地构建/运行、首次 setup、doctor、移除 MCP 集成命令与默认保留数据说明；另列 ChatGPT Desktop/Codex、Claude Code、OpenCode 插件适配及未来官方平台计划，注明当前未发布；
10. `README.md` 与 `README.zh-CN.md` 双语介绍和完整内容的验证结果；
11. 阶段性 Conventional Commits 的 `git log --oneline` 摘要与 `git status`，明确全部使用英文且未推送。

**第一轮开始后，不要只输出一份详细计划便停止。请完成 Stage 0 并立即进入 Stage 1，持续尽可能实现和验证 Stage 2/3/4。每完成可验证的增量立即进行符合规范的本地 Git 提交，并同步改善当前本地使用、setup/doctor/集成移除体验和双语文档；在 `docs/TODO.md` 维护 Plugin 交付计划，当前不发布，独立系统安装包已取消。重要缺陷优先于新特性。**

<div align="center">
<img src="./assets/icons/agent-job-relay.svg" alt="icon" width="200"/>
<h1>AgentJobRelay</h1>

Tasks run in the background. Agents pick up when needed.

任务在后台运行，Agent 在需要时接棒。

[English](README.md) | 简体中文
</div>

让耗时命令独立于 Agent 连接运行，保存结果，并在宿主支持时通知发起任务的聊天。

## 可以做什么

| 能力 | 用途 |
|---|---|
| 持久后台任务 | 运行构建、测试、训练和数据处理，关闭聊天或查看器后仍可继续 |
| Goal 等待（实验性源码） | 活动 goal 在后台运行期间暂停、完成后继续；缺少授权时简短询问一次，同一请求的后续轮次沿用答复。保留原生确认、恢复和删除保护。见[用法及宿主限制](docs/goal-waiting.md) |
| 批量提交（未发布源码） | 通过 CLI/MCP 一次提交 1–16 个独立命令，逐项返回结果并支持安全重试，见[使用指南](docs/usage.md#submit-independent-commands-together-unreleased-source) |
| 远程任务（未发布源码） | 在固定身份的远端运行时启动或附加命令，或观察固定的 GitHub Actions attempt；见[配置与边界](docs/remote-execution.md) |
| 中间反馈（未发布源码） | 结构化阶段、步数和指标，重复里程碑或阈值告警，以及有界 checker 示例；见[用法与边界](docs/progress.md) |
| 结果与历史 | 保存有界的 stdout/stderr、退出结果、事件和任务历史 |
| 绑定会话的通知 | 将结果投递到发起会话，在宿主支持时续接聊天 |
| 本地任务面板 | 通过 CLI 或随包 Web UI 检查任务、设置输出/静默/时长规则并停止任务 |
| 数据维护 | 自动归档、滚动备份，以及显式恢复到新数据目录 |

**运行时支持 Linux x86_64/glibc 和 Windows x86_64。** ChatGPT Desktop 同聊天续接已在两者上通过实验性私有接口验证，Windows 验证使用 Full access 权限。Codex CLI 和 Claude Code 开发框架提供收件箱，不会自动继续聊天。OpenCode V1 属于实验性支持，实际加载器已配合模拟模型验证。macOS/ARM 运行时暂不支持。能力说明见[功能与支持范围](docs/features.md)，已验证版本见[真实验证记录](docs/live-e2e.md)。

## 安装与更新

在线安装需要 Node.js 20+/npm 和网络。ChatGPT Desktop/Codex 还需要 PATH 中可用的 Codex CLI 和获取目录所需的 Git，Desktop 需要本地执行环境，无需 Rust 编译器。以下命令使用 Bash；另有 [PowerShell 和 Release ZIP 安装方式](docs/installation.md#chatgpt-desktop--codex)。

更新时保留原数据目录。当前源码会在新版运行入口首次使用时自动接替旧 daemon，本地运行时更新也会自动重启在线 daemon；替换文件或迁移 schema 前须完成活动任务。已发布版本仍需手动停止同一数据目录的 daemon，见[更新前的准备](docs/updating.md#before-updating)。

本地／源码插件通过稳定的指引入口和经过校验的 MCP 运行时转发跟随更新。旧入口需[一次迁移和重连](docs/updating.md#cache-inspection-and-refresh)；以后同版本本地更新也会使用当前流程，无需清理插件缓存。

### ChatGPT Desktop

**在线安装：** 注册与最新已发布运行时匹配的 marketplace：

```bash
AJR_VERSION="$(npm view agent-job-relay-runtime@latest version --registry=https://registry.npmjs.org)" &&
codex plugin marketplace add MorningFrog/agent-job-relay --ref "v$AJR_VERSION"
```

重启 ChatGPT Desktop，打开 **Plugins → AgentJobRelay**，安装并启用插件，在新的本地聊天中选择 `agent-job-relay@agent-job-relay`。

**在线更新：** 将 marketplace 注册替换为最新发布的版本标签：

```bash
AJR_VERSION="$(npm view agent-job-relay-runtime@latest version --registry=https://registry.npmjs.org)" &&
codex plugin marketplace remove agent-job-relay &&
codex plugin marketplace add MorningFrog/agent-job-relay --ref "v$AJR_VERSION"
```

随后在 Desktop 中刷新或重新安装 AgentJobRelay，并重连 MCP，见[完整在线更新步骤](docs/updating.md#update-the-online-marketplace)。

**本地编译：** [编译并安装本地插件](docs/source-install.zh-CN.md)（[English](docs/source-install.md)）；已有安装按[保留历史的更新步骤](docs/updating.md#unified-install-and-update)操作。

### Codex

**在线安装：** 注册已发布的 marketplace，再安装插件条目：

```bash
AJR_VERSION="$(npm view agent-job-relay-runtime@latest version --registry=https://registry.npmjs.org)" &&
codex plugin marketplace add MorningFrog/agent-job-relay --ref "v$AJR_VERSION" &&
codex plugin add agent-job-relay@agent-job-relay
```

若已注册匹配的 marketplace，仅运行最后一条命令即可。安装后重启或重连 Codex。

**在线更新：** 查询最新发布版本、替换目录注册，再重新安装插件条目：

```bash
AJR_VERSION="$(npm view agent-job-relay-runtime@latest version --registry=https://registry.npmjs.org)" &&
codex plugin marketplace remove agent-job-relay &&
codex plugin marketplace add MorningFrog/agent-job-relay --ref "v$AJR_VERSION" &&
codex plugin add agent-job-relay@agent-job-relay
```

随后重启或重连 Codex。CLI 集成提供工具和收件箱，不会自动继续模型回合，见[完整在线更新步骤](docs/updating.md#update-the-online-marketplace)。

**本地编译：** 按[源码教程](docs/source-install.zh-CN.md)编译安装，并使用其中的[本地 Codex 插件入口](docs/source-install.zh-CN.md#6-在-chatgpt-desktop-中安装并选择正确的插件)；[保留历史的更新](docs/updating.md#unified-install-and-update)继续使用原安装及其记录的数据目录。

### OpenCode

**在线安装（V1）：** 在 `opencode.json` 中添加已发布的插件，保留其他已有条目，然后重启 OpenCode：

```json
{
  "plugin": ["agent-job-relay-opencode-v1@latest"]
}
```

匹配的原生运行时会自动安装。不要重复启用另一个 `agent_job_relay` MCP 条目。

**在线更新：** 保留 `@latest` 条目，完成活动任务后，按 OpenCode 支持的方式刷新包缓存并重启，见 [OpenCode 更新步骤](docs/updating.md#update-opencode-v1)。V1 属于实验性支持，V2 暂不支持。

**本地编译：** 参照 [OpenCode 开发接入](docs/host-plugins.md#opencode-v1-development-setup)及[本地构建和更新说明](docs/host-plugins.md#local-build-and-updates)。该宿主使用独立的插件包和共用原生运行时。

## 日常使用

### 在 Agent 中使用

在宿主中选择已安装的 AgentJobRelay 插件。首次使用时，先让它调用 `job_doctor` 确认 MCP 连通，再直接描述任务：

> 后台执行这个项目的测试，完成后告诉我结果。

日常提示词无需程序路径或会话 ID。插件会绑定真实发起会话、提交任务并保存结果。ChatGPT Desktop 在已验证版本中可自动续接同一聊天；OpenCode V1 的续接仍属实验性支持。Codex CLI 使用收件箱，需要时再让 Agent 检查结果。

也可以提出：

> 打开 AgentJobRelay 的任务面板，让我查看正在运行的任务。

插件可以打开随包 Web UI，无需全局安装 CLI。MCP 提交和同聊天通知恢复见[使用指南](docs/usage.md)，持久规则与停止任务见[监控和 Web UI](docs/runtime-monitoring.md)。

### 命令行与 Web UI

需要终端操作时，单独安装 CLI，再使用在线 ChatGPT/Codex 插件的数据目录打开面板或提交任务：

```bash
npm install --global --ignore-scripts --registry=https://registry.npmjs.org agent-job-relay-runtime@latest
agent-job-relay codex ui
agent-job-relay codex start --timeout 60 -- cargo check
agent-job-relay codex list
agent-job-relay codex inspect <job-id>
```

最后一条命令使用提交后返回的任务 ID；首次提交会自动启动 daemon。OpenCode 使用 `agent-job-relay opencode`，原生离线安装使用其记录的启动器及数据目录。源码新增的 `ajr` 短命令尚待发布，具体见[命令参考](docs/commands.md)和 [CLI 更新步骤](docs/updating.md#update-the-terminal-command)。

命令以当前用户的系统权限执行。任务完成、宿主接收（`accepted_by_host`）和实际观察到 Agent 继续执行是三项独立结果。卸载默认保留任务历史和日志，具体见[卸载指南](docs/uninstalling.md)及[故障排查](docs/troubleshooting.md)。

## 按需要查找文档

**[文档导航 / Documentation index](docs/README.md)** 按实际任务整理全部指南。

| 我想…… | 阅读 |
|---|---|
| 了解功能和宿主支持范围 | [功能说明](docs/features.md)、[当前限制](docs/limitations.md) |
| 安装或接入宿主 | [安装](docs/installation.md)、[Windows](docs/windows.md)、[宿主插件](docs/host-plugins.md) |
| 试用功能测试提示词 | [简体中文](docs/test-prompts.zh-CN.md) · [English](docs/test-prompts.md) |
| 执行任务或查找命令 | [使用指南](docs/usage.md)、[命令参考](docs/commands.md)、[监控](docs/runtime-monitoring.md) |
| 更新、卸载、备份或修复安装 | [更新](docs/updating.md)、[卸载](docs/uninstalling.md)、[数据维护](docs/data-maintenance.md)、[排障](docs/troubleshooting.md) |
| 查看进度和验证情况 | [实施状态](docs/IMPLEMENTATION_STATUS.md)、[实测记录](docs/live-e2e.md)、[路线图](docs/TODO.md)、[变更记录](CHANGELOG.md) |
| 参与开发或理解设计 | [贡献指南](CONTRIBUTING.md)、[测试](docs/testing.md)、[架构](docs/architecture.md)、[协议](docs/protocol.md)、[安全](docs/security.md) |
| 准备维护者发布 | [手动发布指南](docs/release.md) |

欢迎社区测试和 PR，尤其是 Claude Code 和更多宿主版本的验证。项目通过 npm 和自建 marketplace 分发，官方目录审批属于独立事项。采用 [MIT](LICENSE) 许可证。

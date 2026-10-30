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
| Goal 等待（实验性） | 一次限定范围的暂停批准，保留原 goal 恢复和删除保护。源码 0.1.17 新增按需临时恢复、汇报并原生再次暂停的中间反馈；见[用法及宿主限制](docs/goal-waiting.md) |
| 批量提交 | 通过 CLI/MCP 一次提交 1–16 个独立命令，逐项返回结果并支持安全重试，见[使用指南](docs/usage.md#submit-independent-commands-together-0112) |
| 远程任务 | 在固定身份的远端运行时启动或附加命令，或观察固定的 GitHub Actions attempt，包含作业／步骤状态和 attempt 直达链接，见[配置与边界](docs/remote-execution.md) |
| 中间反馈 | 结构化阶段、步数和指标，重复里程碑或阈值告警，以及有界 checker 示例；见[用法与边界](docs/progress.md) |
| 结果与历史 | 保存有界的 stdout/stderr、退出结果、事件和任务历史 |
| 绑定会话的通知 | 将结果投递到发起会话，在宿主支持时续接聊天 |
| 本地任务面板 | 通过 CLI 或随包 Web UI 检查任务、设置输出/静默/时长规则并停止任务 |
| 数据维护 | 自动归档、滚动备份，以及显式恢复到新数据目录 |

**运行时支持 Linux x86_64/glibc 和 Windows x86_64。** ChatGPT Desktop 同聊天续接已在两者上通过实验性私有接口验证，Windows 验证使用 Full access 权限。Codex CLI 和 Claude Code 开发框架提供收件箱，不会自动继续聊天。OpenCode V1 属于实验性支持，实际加载器已配合模拟模型验证。macOS/ARM 运行时暂不支持。能力说明见[功能与支持范围](docs/features.md)，已验证版本见[真实验证记录](docs/live-e2e.md)。

## 安装

在线安装需要 **Node.js 20+/npm**、网络，以及 PATH 中可用的 npm 全局命令目录。ChatGPT Desktop/Codex 还需要 **Codex CLI 和 Git**；Desktop 需要本地执行环境。无需 Rust 编译器，请使用当前用户可写的 npm 安装。[PowerShell 命令及其他安装方式](docs/installation.md)。

### ChatGPT Desktop / Codex

```bash
npm install --global --ignore-scripts --registry=https://registry.npmjs.org agent-job-relay-runtime@latest
ajr update
```

这两条命令安装全局 CLI 和匹配的市场／插件。随后重启或重连 Desktop/Codex，在 Plugins 中启用 **AgentJobRelay**，并在本地聊天中选择 `agent-job-relay@agent-job-relay`。先让它调用 `job_doctor` 确认 MCP 连通。预期结果及宿主要求见[安装指南](docs/installation.md#chatgpt-desktop--codex)。

### 仅使用终端和 Web UI

```bash
npm install --global --ignore-scripts --registry=https://registry.npmjs.org agent-job-relay-runtime@latest
ajr ui
```

`ajr` 使用在线 ChatGPT/Codex 的数据目录；`ajr opencode` 选择 OpenCode。原生离线安装使用其记录的启动器和数据目录。

### OpenCode V1

在 `opencode.json` 中添加以下条目，保留其他已有插件，然后重启 OpenCode：

```json
{
  "plugin": ["agent-job-relay-opencode-v1@latest"]
}
```

插件包含匹配的原生运行时。不要重复启用另一个 `agent_job_relay` MCP 条目。见 [OpenCode 配置与支持边界](docs/installation.md#opencode-v1)。

原生本地插件请按[源码教程](docs/source-install.zh-CN.md)（[English](docs/source-install.md)）或[离线包安装步骤](docs/installation.md#quick-install-from-an-offline-bundle)操作。

## 更新或卸载

在线 ChatGPT Desktop/Codex 安装使用以下命令：

| 操作 | 预览 | 执行 |
|---|---|---|
| 更新 CLI 和插件 | `ajr update --dry-run` | `ajr update` |
| 卸载 CLI 和插件 | `ajr uninstall --dry-run` | `ajr uninstall` |

完成后重启或重连宿主。卸载保留历史、日志及备份；有活动任务时拒绝卸载，除非显式使用 `--force` 先停止任务。这些维护命令需要 CLI 0.1.14 或更新版本；[旧版 CLI](docs/updating.md#update-the-terminal-command) 需先运行上方的 npm 安装命令。

按原安装方式维护：[在线及原生更新](docs/updating.md)、[OpenCode 更新](docs/updating.md#update-opencode-v1)、[卸载和配置移除](docs/uninstalling.md)。更新保留原数据目录；原生离线替换、Windows 包替换或不兼容 schema 迁移前，须先完成活动任务。兼容的 Linux runner 可在在线更新期间继续运行。

## 日常使用

### 在 Agent 中使用

在宿主中选择已安装的 AgentJobRelay 插件。首次使用时，先让它调用 `job_doctor` 确认 MCP 连通，再直接描述任务：

> 后台执行这个项目的测试，完成后告诉我结果。

日常提示词无需程序路径或会话 ID。插件会绑定真实发起会话、提交任务并保存结果。ChatGPT Desktop 在已验证版本中可自动续接同一聊天；OpenCode V1 的续接仍属实验性支持。Codex CLI 使用收件箱，需要时再让 Agent 检查结果。

也可以提出：

> 打开 AgentJobRelay 的任务面板，让我查看正在运行的任务。

宿主支持时，插件会在聊天旁打开随包任务面板。MCP 提交和同聊天通知恢复见[使用指南](docs/usage.md)，持久规则与停止任务见[监控和 Web UI](docs/runtime-monitoring.md)。

### 命令行与 Web UI

安装 CLI 后，使用在线 ChatGPT/Codex 插件的数据目录打开面板或提交任务：

```bash
ajr ui
ajr start --timeout 60 -- cargo check
ajr list
ajr inspect <job-id>
```

最后一条命令使用提交后返回的任务 ID；首次提交会自动启动 daemon。OpenCode 使用 `ajr opencode`，原生离线安装使用其记录的启动器及数据目录，具体见[命令参考](docs/commands.md)。

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

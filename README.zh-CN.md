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
| 结果与历史 | 保存有界的 stdout/stderr、退出结果、事件和任务历史 |
| 绑定会话的通知 | 将结果投递到发起会话，在宿主支持时续接聊天 |
| 本地任务面板 | 通过 CLI 或随包 Web UI 检查任务、设置输出/静默/时长规则并停止任务 |
| 数据维护 | 自动归档、滚动备份，以及显式恢复到新数据目录 |

**运行时支持 Linux x86_64/glibc 和 Windows x86_64。** ChatGPT Desktop 同聊天续接已在两者上通过实验性私有接口验证，Windows 验证使用 Full access 权限。Codex CLI 和 Claude Code 开发框架提供收件箱，不会自动继续聊天。OpenCode V1 属于实验性支持，实际加载器已配合模拟模型验证。macOS/ARM 运行时暂不支持。能力说明见[功能与支持范围](docs/features.md)，已验证版本见[真实验证记录](docs/live-e2e.md)。

## 开始使用

在线安装需要 Node.js 20+/npm 和网络。ChatGPT Desktop/Codex 还需要本地执行环境、PATH 中可用的 Codex CLI，以及获取目录所需的 Git，无需 Rust 编译器。

### ChatGPT Desktop / Codex

在 Bash 中注册与最新已发布运行时匹配的 marketplace：

```bash
AJR_VERSION="$(npm view agent-job-relay-runtime@latest version --registry=https://registry.npmjs.org)" &&
codex plugin marketplace add MorningFrog/agent-job-relay --ref "v$AJR_VERSION"
```

重启 ChatGPT Desktop，打开 **Plugins → AgentJobRelay**，安装并启用插件，在新的本地聊天中选择 `agent-job-relay@agent-job-relay`。先让它调用 `job_doctor`，再交给它任务：

> 后台执行这个项目的测试，完成后告诉我结果。

Codex CLI 用户通过 `codex plugin add agent-job-relay@agent-job-relay` 安装条目，CLI 集成提供工具和收件箱。PowerShell、Release ZIP、OpenCode 及原生离线包的步骤见[安装指南](docs/installation.md)。本地开发使用[源码安装教程](docs/source-install.zh-CN.md)（[English](docs/source-install.md)）。

### 终端与 Web UI

需要全局终端命令时，单独安装 CLI：

```bash
npm install --global --ignore-scripts --registry=https://registry.npmjs.org agent-job-relay-runtime@latest
agent-job-relay codex ui
```

该命令使用在线 ChatGPT/Codex 插件的数据目录，打开随包任务面板；也可以让已选中的插件打开面板。源码新增的 `ajr` 短命令尚待发布，使用已发布包时请运行这里的完整命令。入口及数据目录选择见[命令参考](docs/commands.md)。

## 日常使用

首次提交任务会自动启动 daemon。终端示例：

```bash
agent-job-relay codex start --timeout 60 -- cargo check
agent-job-relay codex list
agent-job-relay codex inspect <job-id>
```

最后一条命令使用提交后返回的任务 ID。日志、MCP 提交和同聊天通知恢复见[使用指南](docs/usage.md)，持久规则与停止任务见[监控和 Web UI](docs/runtime-monitoring.md)。

命令以当前用户的系统权限执行。任务完成、宿主接收（`accepted_by_host`）和实际观察到 Agent 继续执行是三项独立结果。更新时应保留原数据目录，卸载默认保留任务历史和日志，具体见[更新指南](docs/updating.md)、[卸载指南](docs/uninstalling.md)及[故障排查](docs/troubleshooting.md)。

## 按需要查找文档

**[文档导航 / Documentation index](docs/README.md)** 按实际任务整理全部指南。

| 我想…… | 阅读 |
|---|---|
| 了解功能和宿主支持范围 | [功能说明](docs/features.md)、[当前限制](docs/limitations.md) |
| 安装或接入宿主 | [安装](docs/installation.md)、[Windows](docs/windows.md)、[宿主插件](docs/host-plugins.md) |
| 执行任务或查找命令 | [使用指南](docs/usage.md)、[命令参考](docs/commands.md)、[监控](docs/runtime-monitoring.md) |
| 更新、卸载、备份或修复安装 | [更新](docs/updating.md)、[卸载](docs/uninstalling.md)、[数据维护](docs/data-maintenance.md)、[排障](docs/troubleshooting.md) |
| 查看进度和验证情况 | [实施状态](docs/IMPLEMENTATION_STATUS.md)、[实测记录](docs/live-e2e.md)、[路线图](docs/TODO.md)、[变更记录](CHANGELOG.md) |
| 参与开发或理解设计 | [贡献指南](CONTRIBUTING.md)、[测试](docs/testing.md)、[架构](docs/architecture.md)、[协议](docs/protocol.md)、[安全](docs/security.md) |
| 准备维护者发布 | [手动发布指南](docs/release.md) |

欢迎社区测试和 PR，尤其是 Claude Code 和更多宿主版本的验证。项目通过 npm 和自建 marketplace 分发，官方目录审批属于独立事项。采用 [MIT](LICENSE) 许可证。

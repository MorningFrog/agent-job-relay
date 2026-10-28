<div align="center">
<img src="./assets/icons/agent-job-relay.svg" alt="icon" width="200"/>
<h1>AgentJobRelay</h1>

[English](README.md) | 简体中文

Tasks run in the background. Agents pick up when needed.

任务在后台运行，Agent 在需要时接棒。
</div>

让长时间命令独立于 Agent 连接运行，保存结果，并在宿主支持时通知发起任务的聊天。**已发布的包支持 Linux/Windows x86_64。** ChatGPT Desktop 同聊天续接已在 Linux 和 Windows 中完成验证，使用实验性接口；Windows 验证使用 Full access 权限。macOS 运行时暂不支持。OpenCode V1 提供实验性插件候选包；Claude Code 提供仅收件箱通知的开发框架，需要社区验证。

OpenCode/Claude 的开发安装及验证边界见[宿主插件指南](docs/host-plugins.md)。

## 快速安装

直接使用最新发布的 npm 包和对应的 GitHub Release，无需 Rust 编译器或源码构建。在线安装需要 Node.js 20+/npm 和网络；ChatGPT Desktop/Codex 还需本地执行环境、PATH 中可用的 Codex CLI，以及获取目录所需的 Git（也可使用 Release ZIP）。

### ChatGPT Desktop / Codex

在 Bash 中自动查询最新 npm 运行时，并选择对应的已发布 marketplace：

```bash
AJR_VERSION="$(npm view agent-job-relay-runtime@latest version --registry=https://registry.npmjs.org)" &&
codex plugin marketplace add MorningFrog/agent-job-relay --ref "v$AJR_VERSION"
```

成功后重启 ChatGPT Desktop，打开 **Plugins → AgentJobRelay**，安装并启用插件，在新的本地聊天中选择 `agent-job-relay@agent-job-relay`。Codex CLI 用户可执行 `codex plugin add agent-job-relay@agent-job-relay`；CLI 集成提供工具和收件箱，不会自动继续模型回合。

先让选中的插件调用 `job_doctor`，再提交任务。技能出现在聊天中不代表 MCP 已连通。PowerShell 命令、GitHub ZIP 安装及原生离线维护见[安装指南](docs/installation.md)。

### OpenCode V1

在 `opencode.json` 中添加最新发布的插件，然后重启 OpenCode：

```json
{
  "plugin": ["agent-job-relay-opencode-v1@latest"]
}
```

插件会自动安装匹配的原生运行时。OpenCode V1 仍属实验性功能，见[宿主集成及验证范围](docs/host-plugins.md)。已有原生本地安装时，请保留原状态配置并按[维护步骤](docs/installation.md#updating-an-existing-installation)操作。

## Windows

同一套已发布 npm 包支持 Windows x86_64，使用 [PowerShell 快速安装命令](docs/installation.md#chatgpt-desktop--codex)。Windows 会按需启动 daemon，宿主清理限制阻止原生启动时，可使用经过身份验证的同用户本地 CIM 引导。若本机策略禁止 CIM，诊断会提供可复制的宿主外启动命令。原生运行时与真实 Desktop 验证的版本范围见 [Windows 指南](docs/windows.md)及[实测记录](docs/live-e2e.md)。

## 在聊天中使用

在已选择插件的 ChatGPT Desktop 本地聊天中提出任务：

> 后台执行这个项目的测试，完成后告诉我结果。

Agent 会绑定当前聊天、提交任务、结束回合，并在收到通知后读取结果。日常提示词无需程序路径或会话 ID。MCP 不可用时，工作流会明确报告 CLI 降级。绑定需要本地终端；普通 Codex CLI 集成提供通知收件箱，不会自动继续聊天。

首次提交或显式注册会话会自动启动 Daemon。命令以当前用户的系统权限执行。任务完成、宿主接收（`accepted_by_host`）和实际观察到 Agent 继续执行是三项独立结果。首次验收请按[人工测试](docs/manual-testing.md)操作；具体 CLI 命令和恢复方法见[使用指南](docs/usage.md)。

## 更新

先完成活动任务。npm 方式安装的 ChatGPT Desktop/Codex marketplace，按[在线更新命令](docs/installation.md#update-the-online-marketplace)查询最新版、替换旧目录注册，再由宿主刷新或重新安装插件并重连 MCP。OpenCode 的 `@latest` 跟随已发布频道，按宿主支持的方式刷新包缓存后重启。运行中的 daemon 保留已加载程序；不兼容升级需要显式停止同一状态配置对应的 daemon。

原生本地安装使用独立的已记录状态配置，见[保留状态的离线更新](docs/installation.md#updating-an-existing-installation)、[统一维护](docs/installation.md#unified-install-and-update)或[中断操作修复](docs/installation.md#compatibility-and-interrupted-operations)。更新不会自动停止 daemon、切换状态配置或重放任务。

## 卸载

通过宿主禁用或移除插件，再按需要移除 marketplace 或配置条目。历史保存在 npm/插件缓存之外，移除插件不会停止已有 daemon 或清除任务数据。原生本地安装的卸载、预览/应用及已修改或外来文件的保留规则见[安装指南](docs/installation.md#uninstalling)。

## 常见问题

| 现象 | 下一步 |
|---|---|
| 看不到插件或 MCP 工具 | 在宿主中刷新插件并重新连接；技能加载不代表 MCP 已加载。 |
| `agent-job-relay-runtime` 报 `npm E404` | 检查在线入口固定的版本、registry 及网络；独立的原生安装方式见[本地源码安装教程](docs/source-install.zh-CN.md)。 |
| 本地自检通过，但聊天工具不可用 | 从宿主调用 `job_doctor`；本地 stdio 自检不能证明宿主连通。 |
| 更新提示运行时仍在使用 | 等待任务结束，再显式停止原安装对应的 Daemon。 |
| 没有收到通知 | 检查会话和投递状态，必要时做同聊天恢复，见[使用指南](docs/usage.md)。 |
| 所有权检查拒绝修改 | 保留文件，检查报告的改动和备份。 |

## 插件发布

源码及发布附件已公开在 [GitHub](https://github.com/MorningFrog/agent-job-relay)。安装使用 npm `latest` 及对应 GitHub Release，包内部仍固定匹配的运行时依赖。在线 marketplace/OpenCode 安装见[发布指南](docs/release.md#consumer-installation)。已有原生安装继续按[离线方式](docs/installation.md#updating-an-existing-installation)维护。

发布仅限维护者手动执行：**Prepare Release** 构建并验证 Cargo 版本对应的候选包；**Publish Release** 预览 OIDC 或提交同一批文件；**Check Release** 独立核验公开结果。**Release All** 提供统一手动入口，默认 dry-run。正式发布使用 `latest`，Release Notes 来自 CHANGELOG。提交、PR 和 tag 不会自动发布。代码规范见[贡献指南](CONTRIBUTING.md)，具体步骤见[维护者发布指南](docs/release.md)。自建市场与 OpenAI 官方插件库是不同渠道。

## 从源码安装

请按[本地源码安装逐步教程](docs/source-install.zh-CN.md)操作（[English](docs/source-install.md)）。教程涵盖环境准备、获取源码、编译、导出离线包、首次安装或保留历史更新、ChatGPT Desktop 插件选择及真实 MCP 检查，每一步均有预期结果。

安装结果是 **AgentJobRelay Local** 中的原生本地插件，通常引用为 `agent-job-relay@agent-job-relay-local`，源码构建还需要 Node.js 24/npm 生成随包界面；接收已构建原生包的用户无需前端工具或 npm 发布。仅编译成功还未完成安装，必须继续完成离线包安装或更新及 ChatGPT Desktop 步骤。Windows 源码命令见 [Windows 指南](docs/windows.md#build-when-no-bundle-is-available)，开发门禁见[测试指南](docs/testing.md)。

## 独立检查与监控（未发布源码）

0.1.10 源码新增 `inspect [job-id] [--watch]`、持久化 `monitor` 规则、可配置的 `stop --grace/--wait` 和 `ui`。React + Vite 任务面板随原生包附带，daemon 启动后默认启用；`ui` 打开选定状态配置。规则执行不依赖 Agent 或查看器在线。见[命令、预期结果与访问说明](docs/runtime-monitoring.md)。已发布的 0.1.9 尚不包含这些能力。

## 实现与验证情况

| 范围 | 实现情况 | 验证结果与剩余范围 |
|---|---|---|
| Linux 运行时与离线维护 | 已实现 | 自动化执行、生命周期、恢复及安装测试通过。 |
| Windows 运行时与自动启动 | 已实现 | Windows 11 原生测试及普通交互用户下的实际 Codex MCP 冷启动通过。 |
| ChatGPT Desktop 同聊天续接 | Linux 和 Windows 已验证 | 已验证的发布基线可在任务完成后续接发起任务的聊天。Windows 验证使用 Full access 权限，具体版本见[实测记录](docs/live-e2e.md)。 |
| OpenCode V1 | 实验性插件候选版 | 实际加载器、MCP 和会话 API 配合模拟模型验证通过；真实服务商会话、重启恢复及 V2 仍待完成。 |
| Claude Code | 仅收件箱通知的框架 | 合成调用者和 MCP 启动器检查通过；实际 Claude 加载器、账号及模型验证需要社区协助。 |
| npm 运行时与自建市场 | 已发布 | Linux/Windows 安装包与加载器检查及公开 registry 全新安装通过；官方目录批准属于独立流程。 |
| CI 与发布 | 手动准备、发布、检查及 Release All | 原生/安装包 CI、两个 npm 包的 OIDC 发布和 GitHub Release 完成均通过；公开包本地安装验证通过，修复后的独立 Actions 检查待维护者运行。 |
| macOS 运行时 | 未实现 | 仅有可移植部分编译检查，尚不支持原生执行。 |
| 检查、监控与 Web UI | 已在未发布的 0.1.10 源码实现 | Linux/Windows 原生与安装包检查通过，Linux 本地浏览器操作已验证；运行中告警的真实宿主验证仍待完成。 |

当前版本新增统一的 `plugin install` 和 `connect [--recover [--apply]]` 入口、聊天与工作目录绑定对象，以及默认精简的 MCP 返回（`full=true` 查看完整记录），见[日常使用](docs/usage.md)和[统一维护](docs/installation.md#unified-install-and-update)。同时减少空闲时的数据库访问，并允许最多四个聊天并发接收通知。当前结果与版本限制统一维护在[实施状态](docs/IMPLEMENTATION_STATUS.md)及[真实验证记录](docs/live-e2e.md)。官方目录投稿需要单独批准，独立系统安装包及软件仓库计划已取消。**欢迎社区测试和 PR，尤其是 Claude Code 及容器化版本验证。**

未发布的当前源码新增自动数据维护：按 30 天、1,000 个热任务及 256 MiB 热日志预算归档，每天轮换备份，并继续支持归档历史查询。同时为控制请求保留数据库读取通道，将后台对账移出 IPC 接收循环。这些改动使用 schema v9；升级前需完成旧任务并显式停止旧 daemon，已发布的安装包尚未包含这些改动。见[自动维护与恢复](docs/data-maintenance.md)。

## 文档与支持范围

| 主题 | 文档 |
|---|---|
| 从源码逐步安装本地插件 | [中文教程](docs/source-install.zh-CN.md)、[English](docs/source-install.md) |
| 安装、状态配置、更新、缓存及移除 | [安装指南](docs/installation.md) |
| 自动容量预算、备份与恢复（未发布源码） | [数据维护](docs/data-maintenance.md) |
| CLI、MCP、通知和恢复 | [使用指南](docs/usage.md) |
| 人工验收步骤和预期结果 | [人工测试](docs/manual-testing.md) |
| 自动化测试和验证边界 | [测试指南](docs/testing.md) |
| 当前支持及证据 | [实施状态](docs/IMPLEMENTATION_STATUS.md)、[真实验证记录](docs/live-e2e.md) |
| 安全和运行限制 | [安全说明](docs/security.md)、[限制](docs/limitations.md) |
| 设计和接口细节 | [架构](docs/architecture.md)、[协议](docs/protocol.md) |
| 可编辑图标及 SVG/PNG 导出 | [图标源文件与导出流程](assets/icons/README.md) |
| 宿主框架及社区测试 | [宿主插件](docs/host-plugins.md)、[参与贡献](CONTRIBUTING.md) |
| 发布候选包和手动 Actions | [发布指南](docs/release.md) |
| 计划及变更 | [路线图](docs/TODO.md)、[变更记录](CHANGELOG.md) |

采用 [MIT 许可](LICENSE)。

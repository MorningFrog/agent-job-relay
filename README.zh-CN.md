<div align="center">
<img src="./assets/icons/agent-job-relay.svg" alt="icon" width="200"/>
<h1>AgentJobRelay</h1>

[English](README.md) | 简体中文

Tasks run in the background. Agents pick up when needed.

任务在后台运行，Agent 在需要时接棒。
</div>

让长时间命令独立于 Agent 连接运行，保存结果，并在宿主支持时通知发起任务的聊天。**v0.1.8 是尚未发布的 Linux/Windows x86_64 开发版本。** ChatGPT Desktop 同聊天续接已在 Linux 和 Windows 中完成验证，使用实验性接口；Windows 验证使用 Full access 权限。macOS 运行时暂不支持。OpenCode V1 提供实验性插件候选包；Claude Code 提供仅收件箱通知的开发框架，需要社区验证。

OpenCode/Claude 的开发安装及验证边界见[宿主插件指南](docs/host-plugins.md)。

## 快速安装

ChatGPT Desktop 使用适合当前 Linux 系统、来源可信的离线包。已有安装请直接按[更新](#更新)操作。首次安装时，在收到的离线包目录中打开终端，执行：

```bash
./relay plugin bootstrap --dry-run
./relay plugin bootstrap --apply
```

这会安装原生运行时并登记本地插件 marketplace，无需 Rust、Python 或 Node。默认仅预览，`--apply` 才写入安装文件和所属配置。当前没有公开下载或自动在线安装器。如果只有源码，请按[从源码安装](#从源码安装)操作。

重启 ChatGPT Desktop，打开 Plugins，选择本地 marketplace（新建时名为 **AgentJobRelay Local**），安装并启用 AgentJobRelay。打开新的本地聊天，选择 `agent-job-relay@agent-job-relay-local`（已有个人市场使用其原名称）。另一个 `agent-job-relay@agent-job-relay` 入口依赖尚未发布的 npm 运行时，不能作为当前可用的本地安装。宿主界面的发现与安装仍需在实际宿主版本上人工验收。

检查默认安装：

```bash
AJR_INSTALL_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/agent-job-relay"
"$AJR_INSTALL_ROOT/relay" --version
"$AJR_INSTALL_ROOT/relay" doctor --client plugin --check-mcp
```

自定义安装请使用原目录。保留该目录：MCP 和缓存中的终端启动器都使用其中受管理的运行时。状态及历史另行保存，安装不修改 PATH 或系统服务。已经安装时，见[更新](#更新)。自定义路径和独立 MCP 配置见[安装指南](docs/installation.md)。

## Windows

使用可信的 Windows 离线 bundle 及其中的 `relay.cmd` 启动器。原生执行、进程树监管、MCP、离线维护及 npm 安装后的宿主加载已通过 Windows 11 验证。0.1.8 候选包同时携带 Linux 与 Windows 程序，尚未发布。Windows 现已支持按需自动启动 daemon；Codex MCP 清理限制下会使用经过身份验证的同用户本地 CIM 引导。若本机策略禁止 CIM，诊断会提供可直接复制的宿主外启动命令。安装、任务示例及运行边界见 [Windows 指南](docs/windows.md)。

## 在聊天中使用

在已选择插件的 ChatGPT Desktop 本地聊天中提出任务：

> 后台执行这个项目的测试，完成后告诉我结果。

Agent 会绑定当前聊天、提交任务、结束回合，并在收到通知后读取结果。日常提示词无需程序路径或会话 ID。MCP 不可用时，工作流会明确报告 CLI 降级。绑定需要本地终端；普通 Codex CLI 集成提供通知收件箱，不会自动继续聊天。

首次提交或显式注册会话会自动启动 Daemon。命令以当前用户的系统权限执行。任务完成、宿主接收（`accepted_by_host`）和实际观察到 Agent 继续执行是三项独立结果。首次验收请按[人工测试](docs/manual-testing.md)操作；具体 CLI 命令和恢复方法见[使用指南](docs/usage.md)。

## 更新

先让活动任务结束。使用**新离线包中的启动器**，这样旧安装尚不认识插件管理命令时也能更新。将 AJR_BUNDLE 设为收到的离线包绝对路径，AJR_INSTALL_ROOT 设为原安装目录：

```bash
"$AJR_BUNDLE/relay" plugin update --root "$AJR_INSTALL_ROOT" --from "$AJR_BUNDLE" --dry-run
"$AJR_BUNDLE/relay" daemon stop --install-root "$AJR_INSTALL_ROOT"
"$AJR_BUNDLE/relay" plugin update --root "$AJR_INSTALL_ROOT" --from "$AJR_BUNDLE" --apply
"$AJR_INSTALL_ROOT/relay" --version
```

更新保留原状态、历史和备份，拒绝降级，不会隐式停止 Daemon。之后通过宿主刷新或重新安装插件，并重新连接 MCP。旧缓存需要先做这次宿主刷新；显式核验后的缓存修复见[安装指南](docs/installation.md#cache-inspection-and-refresh)。

安装、升级或缓存替换中断后，使用新离线包的启动器预览并完成已记录的替换：

```bash
"$AJR_BUNDLE/relay" plugin repair --path "$AJR_INSTALL_ROOT" --dry-run
"$AJR_BUNDLE/relay" plugin repair --path "$AJR_INSTALL_ROOT" --apply
```

修复缓存时改为该缓存的确切目录。修复保留历史和备份，拒绝覆盖修改过的文件，也不会停止 daemon。之后重试原 bootstrap/update 命令，完成剩余步骤。限制见[中断操作恢复](docs/installation.md#compatibility-and-interrupted-operations)。

## 卸载

先在宿主中移除或禁用插件，结束活动任务，再使用同一个已安装启动器：

```bash
"$AJR_INSTALL_ROOT/relay" integration remove plugin --dry-run
"$AJR_INSTALL_ROOT/relay" integration remove plugin --apply
"$AJR_INSTALL_ROOT/relay" daemon stop
"$AJR_INSTALL_ROOT/relay" uninstall --root "$AJR_INSTALL_ROOT" --dry-run
"$AJR_INSTALL_ROOT/relay" uninstall --root "$AJR_INSTALL_ROOT" --apply
```

卸载保留任务数据和备份；遇到已修改或未知安装文件时会拒绝操作。宿主缓存由宿主移除，目前没有数据清除命令。独立 MCP 的移除方法见[安装指南](docs/installation.md#uninstalling)。

## 常见问题

| 现象 | 下一步 |
|---|---|
| 看不到插件或 MCP 工具 | 在宿主中刷新插件并重新连接；技能加载不代表 MCP 已加载。 |
| `agent-job-relay-runtime` 报 `npm E404` | 选中了在线候选入口。按[本地源码安装教程](docs/source-install.zh-CN.md)操作，再选择 `agent-job-relay@agent-job-relay-local`。 |
| 本地自检通过，但聊天工具不可用 | 从宿主调用 `job_doctor`；本地 stdio 自检不能证明宿主连通。 |
| 更新提示运行时仍在使用 | 等待任务结束，再显式停止原安装对应的 Daemon。 |
| 没有收到通知 | 检查会话和投递状态，必要时做同聊天恢复，见[使用指南](docs/usage.md)。 |
| 所有权检查拒绝修改 | 保留文件，检查报告的改动和备份。 |

## 插件发布候选版

源码已公开在 [GitHub](https://github.com/MorningFrog/agent-job-relay)。OpenCode npm 插件、共用运行时包和 ChatGPT 自建插件市场已提供候选实现；**两个 npm 主包尚未发布，目前不能使用正常的在线安装命令。** 现有安装继续按[离线方式](#快速安装)维护。

GitHub 的 **Release** 工作流只能手动运行：dry-run 从 Cargo 读取版本并构建验证候选包，再由另一次手动运行将同一批文件发布到 npm 和 GitHub。首次使用 stage-only npm 权限并由维护者完成 2FA 批准，后续使用 OIDC；Release Notes 从 CHANGELOG 提取。提交、PR 和 tag 不会自动发布。配置、批准、重试及未来在线安装步骤见[发布指南](docs/release.md)。自建市场与 OpenAI 官方插件库是不同渠道。

## 从源码安装

请按[本地源码安装逐步教程](docs/source-install.zh-CN.md)操作（[English](docs/source-install.md)）。教程涵盖环境准备、获取源码、编译、导出离线包、首次安装或保留历史更新、ChatGPT Desktop 插件选择及真实 MCP 检查，每一步均有预期结果。

安装结果是 **AgentJobRelay Local** 中的原生本地插件，通常引用为 `agent-job-relay@agent-job-relay-local`，无需 npm 发布或 Node.js。仅编译成功还未完成安装，必须继续执行 bootstrap/update 及 ChatGPT Desktop 步骤。Windows 源码命令见 [Windows 指南](docs/windows.md#build-when-no-bundle-is-available)，开发门禁见[测试指南](docs/testing.md)。

## 实现与验证情况

| 范围 | 实现情况 | 验证结果与剩余范围 |
|---|---|---|
| Linux 运行时与离线维护 | 已实现 | 自动化执行、生命周期、恢复及安装测试通过。 |
| Windows 运行时与自动启动 | 已实现 | Windows 11 原生测试及普通交互用户下的实际 Codex MCP 冷启动通过。 |
| ChatGPT Desktop 同聊天续接 | Linux 和 Windows 已验证 | v0.1.8 在任务完成后可续接发起任务的聊天。Windows 验证使用 Full access 权限，见[实测记录](docs/live-e2e.md)。 |
| OpenCode V1 | 实验性插件候选版 | 实际加载器、MCP 和会话 API 配合模拟模型验证通过；真实服务商会话、重启恢复及 V2 仍待完成。 |
| Claude Code | 仅收件箱通知的框架 | 合成调用者和 MCP 启动器检查通过；实际 Claude 加载器、账号及模型验证需要社区协助。 |
| npm 运行时与自建市场 | 候选包已准备，未发布 | Linux/Windows 上使用本地回环 registry 的安装与加载器检查通过；公开 registry 安装待验证。 |
| CI 与发布 | 已定义 Linux/Windows 原生检查及手动发布流程 | 早前远程 CI 通过；更新后的 Windows 原生及发布任务尚待远程运行，统一发布流程尚待首次远程手动运行。 |
| macOS 运行时 | 未实现 | 仅有可移植部分编译检查，尚不支持原生执行。 |
| 输出监控/中断 | 已暂缓 | 等待明确的实现请求；当前可读取日志和手动停止任务。 |

当前结果与版本限制统一维护在[实施状态](docs/IMPLEMENTATION_STATUS.md)及[真实验证记录](docs/live-e2e.md)。官方目录投稿需要单独批准，独立系统安装包及软件仓库计划已取消。**欢迎社区测试和 PR，尤其是 Claude Code 及容器化版本验证。**

## 文档与支持范围

| 主题 | 文档 |
|---|---|
| 从源码逐步安装本地插件 | [中文教程](docs/source-install.zh-CN.md)、[English](docs/source-install.md) |
| 安装、状态配置、更新、缓存及移除 | [安装指南](docs/installation.md) |
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

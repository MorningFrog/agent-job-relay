<div align="center">
<img src="./assets/icons/agent-job-relay.svg" alt="icon" width="200"/>
<h1>AgentJobRelay</h1>

[English](README.md) | 简体中文

Tasks run in the background. Agents pick up when needed.

任务在后台运行，Agent 在需要时接棒。
</div>

让长时间命令独立于 Agent 连接运行，保存结果，并在宿主支持时通知发起任务的聊天。**v0.1.6 是尚未发布的 Linux x86_64 开发版本。** Desktop 唤醒使用实验性接口；目前不支持 Windows/macOS。OpenCode V1 提供实验性插件候选包；Claude Code 提供仅收件箱通知的开发框架，需要社区验证。

OpenCode/Claude 的开发安装及验证边界见[宿主插件指南](docs/host-plugins.md)。

## 插件发布候选版

源码已公开在 [GitHub](https://github.com/MorningFrog/agent-job-relay)。OpenCode npm 插件、共用运行时包和 ChatGPT 自建插件市场已提供候选实现；**npm 账号等待验证、包名尚未最终确认，目前不能使用正常的在线安装命令。** 现有安装继续按下方离线方式维护。

GitHub 的 **Release candidate** 工作流只能手动运行：默认生成候选文件，也可选择创建待人工发布的预发行草稿；提交、PR 和 tag 不会自动发布。构建、安装验证及未来在线安装步骤见[发布指南](docs/release.md)。自建市场与 OpenAI 官方插件库是不同渠道。

## 快速安装

Desktop 使用适合当前 Linux 系统、来源可信的离线包。已有安装请直接按[更新](#更新)操作。首次安装时，在收到的离线包目录中打开终端，执行：

```bash
./relay plugin bootstrap --dry-run
./relay plugin bootstrap --apply
```

这会安装原生运行时并登记本地插件 marketplace，无需 Rust、Python 或 Node。默认仅预览，`--apply` 才写入安装文件和所属配置。当前没有公开下载或自动在线安装器。如果只有源码，见[构建离线包](#构建离线包)。

重启 Desktop，打开 Plugins，选择本地 marketplace（新建时名为 **AgentJobRelay Local**），安装并启用 AgentJobRelay。打开新的本地聊天并选择插件。宿主界面的发现与安装仍需在实际宿主版本上人工验收。

检查默认安装：

```bash
AJR_INSTALL_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/agent-job-relay"
"$AJR_INSTALL_ROOT/relay" --version
"$AJR_INSTALL_ROOT/relay" doctor --client plugin --check-mcp
```

自定义安装请使用原目录。保留该目录：MCP 和缓存中的终端启动器都使用其中受管理的运行时。状态及历史另行保存，安装不修改 PATH 或系统服务。已经安装时，见[更新](#更新)。自定义路径和独立 MCP 配置见[安装指南](docs/installation.md)。

## 在聊天中使用

在已选择插件的 Desktop 本地聊天中提出任务：

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
| 本地自检通过，但聊天工具不可用 | 从宿主调用 `job_doctor`；本地 stdio 自检不能证明宿主连通。 |
| 更新提示运行时仍在使用 | 等待任务结束，再显式停止原安装对应的 Daemon。 |
| 没有收到通知 | 检查会话和投递状态，必要时做同聊天恢复，见[使用指南](docs/usage.md)。 |
| 所有权检查拒绝修改 | 保留文件，检查报告的改动和备份。 |

## 构建离线包

开发者或尚无离线包的用户，可在仓库根目录使用 Rust stable、Cargo 和 C 工具链构建。声明的最低 Rust 1.89 尚未独立验证，本地检查使用 1.99.1。

```bash
cargo build --release --locked
AJR_BUNDLE="$PWD/.tmp/agent-job-relay-offline"
./target/release/agent-job-relay plugin pack --output "$AJR_BUNDLE" --dry-run
./target/release/agent-job-relay plugin pack --output "$AJR_BUNDLE" --apply
```

每次导出使用新目录。然后按快速安装运行其中的 `relay plugin bootstrap`，或保留可执行权限后转交整个目录。它是本地交付产物，不是 marketplace ZIP 或已发布版本。构建门禁和隔离加载器检查见[开发验证](docs/testing.md)。

## 文档与支持范围

| 主题 | 文档 |
|---|---|
| 安装、状态配置、更新、缓存及移除 | [安装指南](docs/installation.md) |
| CLI、MCP、通知和恢复 | [使用指南](docs/usage.md) |
| 人工验收步骤和预期结果 | [人工测试](docs/manual-testing.md) |
| 自动化测试和验证边界 | [测试指南](docs/testing.md) |
| 当前支持及证据 | [实施状态](docs/IMPLEMENTATION_STATUS.md)、[真实验证记录](docs/live-e2e.md) |
| 安全和运行限制 | [安全说明](docs/security.md)、[限制](docs/limitations.md) |
| 设计和接口细节 | [架构](docs/architecture.md)、[协议](docs/protocol.md) |
| 宿主框架及社区测试 | [宿主插件](docs/host-plugins.md)、[参与贡献](CONTRIBUTING.md) |
| 发布候选包和手动 Actions | [发布指南](docs/release.md) |
| 计划及变更 | [路线图](docs/TODO.md)、[变更记录](CHANGELOG.md) |

Linux 执行、离线插件管理和 Codex CLI 0.164.0 加载器已有隔离自动化验证。当前远程 CI 已通过 Linux 运行时/候选包检查，以及 macOS/Windows 可移植代码编译检查。Desktop 26.1021.22027 与插件 0.1.5 已通过运行中任务的重启、重绑定和同聊天续聊验证，恢复、事件及投递记录一致。界面启用、版本和重启由用户确认；桥接身份变化已测量，并已观察到 Agent 续聊。全新界面安装及其他宿主版本仍未验证。共用 npm 原生运行时和自建市场候选包已准备但尚未发布；官方目录投稿及输出监控/中断仍待实施。独立系统安装包和软件仓库计划已取消。

OpenCode 1.19.4 已使用真实加载器、MCP 和会话 API 配合本地模拟模型验证；真实模型服务及 V2 仍需验证。Claude Code 仅完成模拟调用者助手与实际 MCP 启动入口检查，没有 Claude 账号、加载器或模型会话实测。**欢迎社区协助测试和提交 PR，尤其是 Claude Code 与容器化版本验证。** 参与方式见[贡献指南](CONTRIBUTING.md)。

MIT 许可。项目独立实现，只读研究 Vanth 的行为需求，见[参考审查](docs/reference-audit.md)。

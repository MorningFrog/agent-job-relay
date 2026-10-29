# 从源码安装本地插件

[文档导航](README.md) · [已发布版本安装](installation.md) · [更新指南](updating.md)

[English](source-install.md) | 简体中文

本教程将 Linux x86_64/glibc 原生程序安装到稳定的用户目录，并登记个人 **AgentJobRelay Local** 市场。新建市场时，在 ChatGPT Desktop 中应选择 **`agent-job-relay@agent-job-relay-local`**。仅完成编译不会安装或启用插件。

仓库自带的 `.agents/plugins/marketplace.json` 对应另一个在线候选入口 `agent-job-relay@agent-job-relay`。它的 MCP 启动器固定源码对应的 npm 运行时，该版本可能尚未发布；在线安装应使用已发布的 Release 对应目录，并使用独立状态配置。本地安装教程不需要把这个源码仓库添加为 marketplace，源码构建需要 Node.js 24/npm 生成 React + Vite 资源；已构建原生包的接收者不需要这些工具。

Windows 请使用 [Windows 指南](windows.md)中的构建和 bootstrap 命令。macOS 和 ARM 尚不支持原生运行。

## 1. 检查准备条件

使用带 glibc 的 Linux x86_64、Bash、Git、Node.js 24/npm、Rust stable/Cargo、用于构建内置 SQLite 的 C 工具链，以及支持本地执行的 ChatGPT Desktop。先检查构建工具：

```bash
git --version
rustc --version
cargo --version
node --version
npm --version
cc --version
```

预期：每条命令都能显示已安装的工具版本。声明的最低 Rust 1.89 尚未独立验证，当前构建证据见[实施状态](IMPLEMENTATION_STATUS.md)。缺少工具时，先按对应工具的正常安装说明补齐，再继续。

## 2. 进入源码目录

尚未获取源码时：

```bash
git clone https://github.com/MorningFrog/agent-job-relay.git
cd agent-job-relay
```

已经有源码时，直接在该仓库根目录打开终端，不要覆盖已有目录。后续构建和导出命令均在包含 `Cargo.toml` 的目录中执行。

## 3. 编译并导出本地离线包

```bash
npm ci --prefix web --ignore-scripts --no-audit --no-fund
npm run build --prefix web
cargo build --release --locked
./target/release/agent-job-relay --version
AJR_BUNDLE="$PWD/.tmp/agent-job-relay-offline-$(date +%Y%m%d-%H%M%S)"
./target/release/agent-job-relay plugin pack --output "$AJR_BUNDLE" --dry-run
./target/release/agent-job-relay plugin pack --output "$AJR_BUNDLE" --apply
```

预期：程序版本与 `Cargo.toml` 一致；pack 输出所选目录、版本及原生平台。导出目录中有 `relay`、`bundle.json` 和 `payload/`，不携带用户状态配置。每次导出都需要尚不存在的目录；时间戳用于下一次运行时生成新名称。保留当前终端，使 AJR_BUNDLE 变量在后续步骤仍然可用。

## 4. 安装或更新受管理的运行时

设置安装目录。Linux 标准默认目录为：

```bash
AJR_INSTALL_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/agent-job-relay"
```

如果以前安装在其他位置，请将 AJR_INSTALL_ROOT 设为那个原始安装目录的绝对路径。个人市场要求安装目录位于 HOME 内；若 XDG_DATA_HOME 位于 HOME 外，首次安装可选择 `$HOME/.local/share/agent-job-relay` 等 HOME 内的目录。安装目录、导出的离线包及任务状态目录必须分开。

### 首次安装

仅在所选目录中没有既有安装时执行这一分支：

```bash
"$AJR_BUNDLE/relay" plugin install --root "$AJR_INSTALL_ROOT" --dry-run
"$AJR_BUNDLE/relay" plugin install --root "$AJR_INSTALL_ROOT" --apply
```

预期：预览显示正确的 `install.root`、`install.state_dir` 和 `marketplace.config_path`，核对后再 apply。新安装的状态目录优先使用已设置的 AJR_STATE_DIR，否则使用 XDG/HOME 默认值。Apply 安装原生程序，并在 `$HOME/.agents/plugins/marketplace.json` 中登记插件，保留其他条目。新建市场的名称为 `agent-job-relay-local`，显示为 **AgentJobRelay Local**；已有市场会保留原名称。

### 已有安装

更新既有运行时时，执行这一分支，不要重复首次安装分支。先让活动任务结束，再执行：

```bash
"$AJR_BUNDLE/relay" plugin install --root "$AJR_INSTALL_ROOT" --from "$AJR_BUNDLE" --dry-run
"$AJR_BUNDLE/relay" plugin install --root "$AJR_INSTALL_ROOT" --from "$AJR_BUNDLE" --apply
```

预期：预览保留原状态目录，apply 保留历史并报告生成的备份。apply 会在检查活动任务后自动停止在线 daemon，并用更新后的安装启动新版；原本离线的 profile 保持离线。遇到活动任务、所有权或降级错误时，应停止并诊断，不要换用新状态目录绕过错误。这一步更新的是受管理运行时；ChatGPT Desktop 插件及缓存仍需执行第 6 步。

## 5. 检查已安装入口

```bash
"$AJR_INSTALL_ROOT/relay" --version
"$AJR_INSTALL_ROOT/relay" doctor --client plugin --check-mcp
```

预期：已安装版本与构建版本一致，受管理的 marketplace 条目匹配，`stdio_self_check.status` 为 `passed`，当前源码工具数为 24（已发布 0.1.10／原封存 0.1.11 为 22）。首次提交任务前 daemon 处于 stopped 状态是正常的。该检查不提交任务，只验证本机 stdio 工具发现，不能证明 ChatGPT Desktop 已连接或聊天能自动续接。

请保留 AJR_INSTALL_ROOT：缓存中的 MCP 和终端启动器都使用该目录中的原生程序。仅重新编译 `target/release` 不会更新已安装运行时。任务状态和历史另行保存；[状态配置](installation.md#custom-directories-and-stored-data)、[更新](updating.md)和[卸载](uninstalling.md)分别见对应指南。

## 6. 在 ChatGPT Desktop 中安装并选择正确的插件

已有插件若仍缓存完整旧技能，需要使用宿主／插件安装返回的**准确缓存目录**执行一次迁移，再按下文重连。不要传入缓存父目录，也不要搜索或删除其他插件：

```bash
AJR_CACHE="<宿主返回的缓存绝对目录>"
"$AJR_INSTALL_ROOT/relay" plugin cache refresh --path "$AJR_CACHE" --dry-run
"$AJR_INSTALL_ROOT/relay" plugin cache refresh --path "$AJR_CACHE" --apply
"$AJR_INSTALL_ROOT/relay" plugin status --cache "$AJR_CACHE"
```

预期：`matches_installed=true`，并保留备份。新缓存技能每次调用都会通过 `relay instructions` 读取当前流程，受管 MCP 长连接会核对安装后使用后续更新的运行时。完成这次迁移和重连后，同版本本地源码更新也不必再手动刷新这些入口。在线固定版本 npm 包仍按其发行更新路线处理。

1. 重启 ChatGPT Desktop，打开 **Plugins**。
2. 选择个人市场 **AgentJobRelay Local**，安装并启用其中的 **AgentJobRelay**。若个人市场原来有其他名称，按 `$HOME/.agents/plugins/marketplace.json` 中的实际名称选择。
3. 如果在线候选入口 `agent-job-relay@agent-job-relay` 已启用，先通过 ChatGPT Desktop 停用该候选入口，再验收本地插件，保留其他插件不变。
4. 新建一个**本地聊天**，输入 `@AgentJobRelay`，选择来自本地市场的条目。新建市场时，其引用为 `plugin://agent-job-relay@agent-job-relay-local`。

如果已经安装 Codex CLI，新建本地市场的显式安装命令为：

```bash
codex plugin add agent-job-relay@agent-job-relay-local
```

然后仍需按上文重启或重连 ChatGPT Desktop，并打开新的本地聊天。CLI 安装本身不能证明 ChatGPT Desktop 的 MCP 可用。界面名称及可用性取决于宿主版本；若看不到本地市场，应检查第 5 步及市场文件路径，不要改为添加本仓库的在线市场。

## 7. 先验证 MCP，再提交长任务

选中本地插件后，先发送：

> 调用 AgentJobRelay 的 MCP job_doctor，验证 managed_mcp_updates_supported=true、goal_wait_policy=required_for_active_desktop_goal，并确认发现的 job_goal_wait 接口包含 confirm_paused。不提交任务。如果没有它的 MCP 工具，请报告并停止，不使用 CLI 降级，也不使用其他插件的工具。

预期：实际调用 AgentJobRelay 的 `job_doctor` 成功且具备上述能力。排查同版本更新时，可将其 `runtime_workflow_sha256` 与所选启动器 `instructions` 返回的摘要比较；仅版本号相同不足以证明更新生效。共享 MCP 中，`binding_mode=explicit current-terminal binding` 和 `desktop_context.available=false` 是正常结果；真正提交任务时才会通过终端绑定当前聊天。

成功后，再使用下一项实际需要的长任务：

> 通过已选中的本地 AgentJobRelay 插件的 MCP 后台执行本项目的全量测试。绑定当前聊天，提交后结束本轮；完成后在当前聊天继续汇报。如果 MCP 不可用，请报告并停止本次验收，不使用 CLI 降级，也不使用其他插件的工具。

预期：Agent 从当前聊天终端取得新的绑定，通过 AgentJobRelay MCP `job_start` 提交，报告任务 ID，然后结束本轮。实际观察到同聊天续接是另一项验收结果，宿主接收本身不能证明续接成功。验证边界见[人工验收](manual-testing.md)。

当前 Desktop 宿主要求用户／系统授权原生 goal 暂停；无需额外措辞的自动交接不受支持，会在提交前拒绝。已有独立暂停请求时可使用实验性的原生确认流程；插件安装或缓存刷新不能授予宿主生命周期控制权限。见 [goal 等待](goal-waiting.md)。

## 能看到技能，但没有 job_start 时

先核对选中的市场入口。若 `agent-job-relay-runtime` 报 `npm E404`，应检查在线入口固定的版本、registry 和网络；本教程仍按第 6 步安装并选择本地市场条目，然后新建聊天。若第 5 步成功但本地条目仍无工具，应刷新或重新安装该条目并重连 ChatGPT Desktop MCP；宿主仍无法加载时停止验收。不要手改缓存清单、移动状态目录，或改用其他插件提交任务来完成验收。

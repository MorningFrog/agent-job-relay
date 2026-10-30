# Documentation / 文档导航

[English home](../README.md) · [中文首页](../README.zh-CN.md)

Choose the task you need below. Installation, everyday operation, maintenance and developer references have separate guides. Version progress and test results live in the status documents linked under [progress and evidence](#progress-and-evidence--进度与验证).

按要完成的任务选择文档。首次使用从安装开始；已有安装按原安装方式更新；版本进度、测试结果和待办分别在下方的状态、实测和路线图中维护。

## Capabilities and support / 功能与支持

| Question / 需要了解 | Guide / 文档 |
|---|---|
| What can it do? / 有哪些功能、支持哪些宿主？ | [Features and support](features.md) |
| What is unsupported or unqualified? / 有哪些限制？ | [Current limitations](limitations.md) |
| What permissions and private data are involved? / 权限、数据和安全边界是什么？ | [Security and data ownership](security.md) |

## Installation and connection / 安装与接入

| Task / 任务 | Guide / 文档 |
|---|---|
| Install published CLI or ChatGPT Desktop/Codex/OpenCode plugin / 安装已发布版本 | [Installation](installation.md) |
| Build and install a native local plugin / 从源码安装本地插件 | [Source walkthrough](source-install.md) · [中文教程](source-install.zh-CN.md) |
| Use Windows, check startup or binding permissions / Windows 安装、启动和权限 | [Windows guide](windows.md) |
| Develop or qualify OpenCode/Claude adapters / 接入和验证其他宿主 | [Host plugins](host-plugins.md) |

## Everyday operation / 日常使用

| Task / 任务 | Guide / 文档 |
|---|---|
| Submit a job, read results or recover same-chat notifications / 提交任务、查看结果、恢复通知 | [Usage walkthrough](usage.md) |
| Pause a goal while Relay jobs run / 后台任务期间暂停 goal | [Goal waiting](goal-waiting.md) (experimental) |
| Launch/attach remote commands or observe a workflow attempt / 远程命令和工作流观察 | [Remote execution](remote-execution.md) (0.1.12+) |
| Look up CLI commands, profiles or MCP equivalents / 查询命令、数据目录和 MCP 工具 | [Command reference](commands.md) |
| Inspect/watch, add rules, use Web UI or stop work / 检查任务、监控、面板和停止 | [Runtime monitoring and Web UI](runtime-monitoring.md) |
| Report stages, metrics or checkpoints during execution / 阶段、指标和检查点中间反馈 | [Progress and checker recipes](progress.md) (0.1.12+) |
| Copy prompts to test features / 复制提示词测试各项功能 | [English](test-prompts.md) · [简体中文](test-prompts.zh-CN.md) |

## Maintenance and troubleshooting / 维护与排障

| Task / 任务 | Guide / 文档 |
|---|---|
| Update online or native installations / 更新已发布包或原生本地安装 | [Updating](updating.md) |
| Repair an interrupted installation or a known cache / 修复中断安装或指定缓存 | [Cache refresh](updating.md#cache-inspection-and-refresh) · [interruption repair](updating.md#compatibility-and-interrupted-operations) |
| Remove CLI, plugin or standalone MCP configuration / 卸载和移除配置 | [Uninstalling](uninstalling.md) |
| Understand archival, configure budgets, back up or restore / 归档、容量、备份和恢复 | [Data maintenance](data-maintenance.md) |
| Diagnose missing tools, old daemons, startup or delivery failures / 定位工具、后台、启动和通知问题 | [Troubleshooting](troubleshooting.md) |

## Progress and evidence / 进度与验证

| Question / 需要了解 | Authoritative document / 对应文档 |
|---|---|
| What is implemented and tested now? / 当前实现和测试到了哪里？ | [Implementation status](IMPLEMENTATION_STATUS.md) |
| What actually ran in a real host? / 哪些已通过真实宿主验证？ | [Current live verification](live-e2e.md) |
| What remains, is deferred or cancelled? / 还有哪些待办、暂缓或取消事项？ | [TODO / roadmap](TODO.md), the sole project backlog |
| What changed in a version? / 每个版本变更了什么？ | [Changelog](../CHANGELOG.md) |

Installation success, tool discovery, host acceptance and observed Agent continuation are distinct evidence. Status documents state the applicable version and limits; older results stay in Git history.

## Development and releases / 开发与发布

| Task / 任务 | Guide / 文档 |
|---|---|
| Contribute a bounded change / 参与贡献 | [Contributing](../CONTRIBUTING.md) · [development contract](../AGENTS.md) |
| Run automated checks / 自动化测试 | [Testing](testing.md) |
| Verify installation and real-host behavior manually / 人工验收 | [Manual acceptance](manual-testing.md) |
| Locate code and understand runtime boundaries / 代码入口与架构 | [Architecture and source map](architecture.md) |
| Inspect MCP, IPC, delivery and compatibility details / 协议与兼容性 | [Protocol reference](protocol.md) |
| Read the governing scope or design rationale / 范围规范与设计决策 | [Project specification](../PROJECT_SPEC.md) · [architecture decision records](adr/) |
| Edit and export the icon / 图标维护 | [Artwork guide](../assets/icons/README.md) |
| Prepare, publish or check exact release candidates / 维护者准备、发布与核验 | [Maintainer release guide](release.md) |

Package entry-point notes: [npm runtime](../packages/runtime/README.md), [ChatGPT Desktop/Codex](../plugins/chatgpt/README.md) and [OpenCode V1](../plugins/opencode-v1/README.md). These describe their own packages; use the task guides above for complete workflows.

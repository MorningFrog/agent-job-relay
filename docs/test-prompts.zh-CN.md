# 任务提示词示例

[English](test-prompts.md) · [文档索引](README.md)

复制一条给 Agent，按需要替换目录、文件名或链接。

## 扫描 Linux 主目录

```text
扫描 /home/$USER 目录，统计文件数量、文件类型和总大小。使用 AgentJobRelay 管理后台任务，完成后告诉我结果。
```

## 扫描 Windows 用户目录

```text
扫描 %USERPROFILE% 目录，统计文件数量、文件类型和总大小。使用 AgentJobRelay 管理后台任务，完成后告诉我结果。
```

## 查找文件

```text
在我的主目录中查找名为 report.pdf 的文件。使用 AgentJobRelay 管理后台搜索，找到后告诉我完整路径。
```

## 查找大文件

```text
找出我的主目录中最大的 20 个文件，列出路径和大小。使用 AgentJobRelay 管理后台任务。
```

## 扫描时反馈进度

```text
扫描我的主目录，统计文件数量和总大小。使用 AgentJobRelay 管理后台任务，每 30 秒告诉我一次进度，完成后给出汇总。
```

## 同时统计多个目录

```text
同时统计我的主目录下各个子目录的大小。使用 AgentJobRelay 管理这些后台任务，完成后按大小排序汇总。
```

## 构建和测试项目

```text
运行当前项目的完整构建和测试。使用 AgentJobRelay 管理后台任务，完成后告诉我结果和失败原因。
```

## 等待扫描时暂停 goal

```text
使用 AgentJobRelay 在后台扫描我的主目录。等待期间暂停当前 goal，扫描完成后恢复 goal 并报告结果。
```

## 观察工作流

```text
使用 AgentJobRelay 在后台观察这个 GitHub Actions 运行：<运行链接>。结束后告诉我结果。
```

## 查看任务进度

```text
打开 AgentJobRelay 任务管理器，查看刚才目录扫描任务的进度。
```

## 停止任务

```text
使用 AgentJobRelay 停止刚才的目录扫描任务，告诉我已经扫描到哪里。
```

## 恢复任务通知

```text
恢复 AgentJobRelay 中刚才后台任务的通知，并告诉我当前结果，不要重新运行任务。
```

---
name: wx-history
description: 拉取并展示与指定联系人的微信聊天历史记录，支持查询、统计和导出
metadata:
  type: tool
  binary: wx-agent
---

## 功能

通过 `wx-cli` 直接读取本地微信数据库，获取历史聊天记录，用于：
- 查看特定联系人的聊天记录
- 统计消息频率和互动规律
- 为蒸馏提供原始数据
- 生成聊天报告

## 使用方法

> **注意**: wx-agent 目前通过 `wx-cli export` 和 `wx-cli history` 子命令读取历史记录。

```bash
# 导出联系人完整聊天记录（wx-cli 直接调用）
wx history <联系人名>        # 最近 N 条
wx export <联系人名>         # 全量导出
wx sessions                  # 列出所有会话
```

通过 wx-agent 间接触发（蒸馏时自动拉取）：
```bash
wx-agent distill contact <联系人名>   # 拉取+蒸馏
```

## 聊天报告（年度/月度）

结合 Hermes 或 OpenClaw，可基于聊天记录生成结构化报告：

1. 激活 `/wx-history`
2. 指定联系人和时间范围
3. AI 分析消息内容，生成报告

示例报告内容：
- 消息总量、日均频率
- 话题分布（工作/生活/情感等）
- 情感曲线（时间维度）
- 关键事件时间线

## 数据来源

- **wx-cli** 直接解密读取 `~/Library/Containers/com.tencent.xinWeChat/` 下的 SQLCipher 数据库
- 读取仅限本地数据，无需网络连接
- 需要微信处于登录状态或有历史数据库

## 在 Hermes / OpenClaw 中

激活后可用自然语言指令，例如：
- "帮我看看和张三的聊天记录"
- "分析我和李四今年的聊天，生成一份年度报告"
- "统计过去一个月我收到消息最多的联系人"

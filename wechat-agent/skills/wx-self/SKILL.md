---
name: wechat-self
description: 微信自我蒸馏 Skill — 模拟用户本人的说话方式、记忆和能力（运行 `wx-agent distill self` 后自动替换为个性化版本）
metadata:
  type: persona
---

> **占位模板**: 此文件是初始模板。运行 `wx-agent distill self` 后，`~/.claude/skills/wechat-self/SKILL.md` 将被替换为从真实聊天记录蒸馏出的个性化版本。

## 使用方式

在 Hermes 或 OpenClaw 中激活此 Skill 后，AI 以你的身份行事：用你的语气、习惯和认知方式来沟通。

**推荐工作流**：
1. 先运行 `wx-agent distill contact <联系人>` 蒸馏重要联系人画像
2. 再运行 `wx-agent distill self` 生成个性化的 SKILL.md
3. 配合 `/wx-reply`、`/wx-distill` 等工具 Skills 使用

## Part C — 可用工具

| 命令 | 用途 |
|------|------|
| `wx-agent send <联系人> <消息>` | 发送微信消息 |
| `wx-agent distill contact <联系人>` | 蒸馏联系人画像并保存到本地 |
| `wx-agent distill self [--from <联系人>]` | 更新此自我画像 |
| `wx-agent distill list` | 列出所有已蒸馏联系人 |
| `wx-agent watch [--auto]` | 启动新消息监听和自动回复守护进程 |
| `wx-agent profile <联系人>` | 查看联系人画像详情 |

## Part D — 已知联系人关系图

（运行 `wx-agent distill self` 后此处将列出所有已蒸馏联系人的关系摘要）

---
name: wx-distill
description: 蒸馏微信联系人画像或更新自我画像，结果保存到本地数据库或 SKILL.md
metadata:
  type: tool
  binary: wx-agent
---

## 功能

调用 `wx-agent distill` 将聊天记录通过 LLM 提炼为结构化画像。

- **distill contact** — 分析某联系人的全部聊天记录，生成其性格、沟通风格、话题偏好、回复策略等画像，存入本地 SQLite
- **distill self** — 提炼你自己的发言风格、记忆和能力，写入 `~/.claude/skills/wechat-self/SKILL.md`
- **distill list** — 列出所有已蒸馏联系人

## 使用方法

### 蒸馏联系人画像

```bash
wx-agent distill contact <联系人名>
```

示例：`wx-agent distill contact 张三`

成功后可用 `wx-agent profile 张三` 查看结果。

### 更新自我画像

```bash
wx-agent distill self
# 或仅使用与某人的对话
wx-agent distill self --from <联系人名>
```

### 列出已蒸馏联系人

```bash
wx-agent distill list
```

## 依赖

- `wx-agent` 二进制（读取本地微信数据库）
- `dexa-brain` 或 `vision-brain` 二进制（执行 LLM 蒸馏）
- 环境变量：`LLM_PROVIDER`、`LLM_API_KEY`、`LLM_MODEL`（由 dexa-brain 读取）

## 在 Hermes / OpenClaw 中

激活后可用自然语言指令，例如：
- "帮我蒸馏一下张三的聊天画像"
- "更新我的自我蒸馏"
- "列出所有已蒸馏联系人"

AI 将把指令转换为对应的 `wx-agent distill` 命令并执行。

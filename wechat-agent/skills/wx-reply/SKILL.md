---
name: wx-reply
description: 检测微信新消息，结合联系人画像生成个性化回复，支持半自动确认或全自动发送
metadata:
  type: tool
  binary: wx-agent
---

## 功能

基于已蒸馏的联系人画像和自我画像，对收到的微信新消息生成符合你风格的回复。

工作流程：
1. `wx-agent watch` 检测新消息，存入本地 SQLite 队列
2. 加载对应联系人的画像（如已蒸馏）
3. 调用 LLM 生成回复草稿
4. 半自动模式：展示草稿等待确认；全自动模式：直接发送

## 使用方法

### 半自动模式（默认，推荐）

```bash
wx-agent watch
```

每次生成回复后会打印草稿并等待你按 `y` 确认或 `n` 跳过。

### 全自动模式

```bash
wx-agent watch --auto
```

直接发送，无需确认。**谨慎使用**。

## 前置条件

- 已运行 `wx-agent distill contact <联系人>` 建立联系人画像
- 已运行 `wx-agent distill self` 建立自我画像（可选，但提升回复质量）
- `config.toml` 中 `require_profile = true`（默认）时，只回复已蒸馏的联系人

## 配置参数（config.toml）

```toml
[agent]
mode           = "semi"   # "semi" | "auto"
poll_interval  = 5        # 轮询间隔（秒）
reply_max_len  = 80       # 回复最大字数
require_profile = true    # 仅回复已蒸馏联系人
```

## 在 Hermes / OpenClaw 中

激活后可用自然语言指令，例如：
- "启动微信自动回复"
- "查看新消息并帮我回复"
- "开启全自动回复模式"

AI 将执行对应的 `wx-agent watch` 命令并汇报状态。

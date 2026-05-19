---
name: wx-watch
description: 管理微信消息监听守护进程的启动、状态查询和配置
metadata:
  type: tool
  binary: wx-agent
---

## 功能

`wx-agent watch` 是一个持续运行的后台守护进程，它：
1. 每隔 N 秒轮询 `wx-cli new-messages` 获取新消息
2. 将未处理消息写入 SQLite 队列（防崩溃丢失）
3. 对每条消息：加载联系人画像 → 调用 LLM 生成回复 → 确认/自动发送

## 使用方法

### 启动监听（半自动）

```bash
wx-agent watch
```

每次生成回复后等待用户确认（`y` 发送，`n` 跳过）。

### 启动监听（全自动）

```bash
wx-agent watch --auto
```

### 推荐启动方式（后台运行）

```bash
# macOS / Linux
nohup wx-agent watch > ~/.wx-agent/watch.log 2>&1 &

# 或使用 tmux
tmux new-session -d -s wx-watch 'wx-agent watch'
```

## 状态检查

```bash
# 查看是否在运行
pgrep -f "wx-agent watch"

# 查看日志
tail -f ~/.wx-agent/watch.log

# 查看待处理消息队列（SQLite）
sqlite3 ~/.wx-agent/data.db "SELECT * FROM pending_messages WHERE status='pending';"
```

## 消息队列说明

所有新消息先写入 `~/.wx-agent/data.db` 的 `pending_messages` 表，状态为 `pending`。
- 已发送回复：`replied`
- 人工跳过：`skipped`
- 重启后继续处理未完成的 `pending` 消息

## 配置参数（config.toml）

```toml
[agent]
mode            = "semi"   # "semi"（需确认）| "auto"（全自动）
poll_interval   = 5        # 轮询间隔，单位秒
reply_max_len   = 80       # 回复最大字数
require_profile = true     # true = 只回复已蒸馏联系人
```

## 在 Hermes / OpenClaw 中

激活后可用自然语言指令，例如：
- "启动微信消息监听"
- "查看微信守护进程状态"
- "停止自动回复"
- "查看待处理消息队列"

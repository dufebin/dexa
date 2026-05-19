# wechat-agent Skills

Hermes / OpenClaw SKILL.md files for the wechat-agent toolchain.

## Skills

| Skill | 用途 |
|-------|------|
| [wx-self](wx-self/SKILL.md) | 你的虚拟自我 — 蒸馏自你的聊天记录，包含自我记忆、语气风格、工具清单和联系人关系图 |
| [wx-distill](wx-distill/SKILL.md) | 蒸馏联系人画像或更新自我画像 |
| [wx-reply](wx-reply/SKILL.md) | 监听新消息并基于画像生成个性化回复 |
| [wx-send](wx-send/SKILL.md) | 通过微信 UI 自动化发送消息 |
| [wx-history](wx-history/SKILL.md) | 拉取聊天历史记录，生成报告 |
| [wx-watch](wx-watch/SKILL.md) | 管理消息监听守护进程 |

## 安装

```bash
./skills/install.sh
# 或指定目录
./skills/install.sh --target ~/.claude/skills
```

## 工作流

### 初始化

```bash
# 1. 蒸馏重要联系人
wx-agent distill contact 张三
wx-agent distill contact 李四

# 2. 生成自我蒸馏（包含联系人关系图）
wx-agent distill self

# 3. 安装 Skills（如未安装）
./skills/install.sh
```

### 日常使用

```bash
# 启动自动回复守护进程
wx-agent watch

# 或在 Hermes/OpenClaw 中激活
/wx-reply
```

## SKILL.md 架构说明

### wx-self（虚拟自我）

`wx-agent distill self` 生成的 `~/.claude/skills/wechat-self/SKILL.md` 包含四部分：

- **Part A — Self Memory**: LLM 从你的发言中提炼的记忆片段、价值观、习惯
- **Part B — Persona**: 语气、说话风格、常用词汇、回复模式
- **Part C — 可用工具**: wx-agent 命令清单（静态）
- **Part D — 联系人关系图**: 所有已蒸馏联系人的关系摘要（动态，每次蒸馏更新）

这使得 Hermes/OpenClaw 中的 `/wechat-self` 不仅知道**你是谁**，还知道**你能做什么**和**你认识谁**。

## 架构图

```
Hermes / OpenClaw
    │
    ├── /wechat-self  ←── ~/.claude/skills/wechat-self/SKILL.md
    │       虚拟自我：记忆 + 语气 + 工具 + 关系
    │
    ├── /wx-distill   ─── wx-agent distill contact/self
    ├── /wx-reply     ─── wx-agent watch
    ├── /wx-send      ─── wx-agent send
    ├── /wx-history   ─── wx history / wx export
    └── /wx-watch     ─── wx-agent watch --auto

wx-agent (Rust, 无 LLM)
    ├── wx-cli        ── 读取微信本地 SQLCipher 数据库
    ├── hand          ── 键鼠 UI 自动化 (desktop-hand)
    └── dexa-brain    ── LLM 后端 (distill / generate-reply)
                          读取: LLM_PROVIDER, LLM_API_KEY, LLM_MODEL
```

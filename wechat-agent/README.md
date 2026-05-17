# wx-agent

微信自动化智能体：读取本地聊天记录 → 蒸馏联系人画像 → 自动生成回复 → 通过 UI 自动化发送消息。

支持 **macOS** 和 **Windows**，全部本地运行，数据不出本机。

---

## 架构

```
wx-agent（调度层）
    │
    ├── wx  (wx-cli)         读取微信本地加密数据库
    │   sessions / history / new-messages / export
    │
    ├── hand (desktop-hand)  跨平台鼠标键盘自动化
    │   key combo / key type / key tap / mouse click
    │
    └── Claude API           LLM 驱动蒸馏与回复生成
        Haiku  → 实时回复生成（高频，低成本）
        Sonnet → 联系人画像蒸馏（质量优先）
```

### 数据流

```
微信本地 SQLCipher DB
    ↓ wx-cli 解密读取
wx new-messages / history
    ↓
wx-agent（SQLite 缓存）
    ↓ 联系人画像
Claude Sonnet（蒸馏）→ contact_profiles
    ↓ 实时回复
Claude Haiku（回复）→ 候选回复
    ↓ UI 自动化
desktop-hand → 在 WeChat 窗口中发送
```

---

## 前置条件

### 1. 配置 wx-cli（无需安装）

本项目内置了 Windows 版 `wx-cli.exe`，存放于 `D:\workspace\dexa\wechat-agent\crates\wx-cli\wx-cli.exe`。`config.toml` 中已默认配置该路径，无需再通过 npm 全局安装。

### 2. 初始化 wx-cli（需要微信在运行）

**Windows**（以管理员身份运行 PowerShell）：
```powershell
D:\workspace\dexa\wechat-agent\crates\wx-cli\wx-cli.exe init
D:\workspace\dexa\wechat-agent\crates\wx-cli\wx-cli.exe sessions
```

**macOS**（若在 macOS 运行，需使用对应的 wx-cli 二进制并对微信做 ad-hoc 签名）：
```bash
codesign --force --deep --sign - /Applications/WeChat.app
killall WeChat && open /Applications/WeChat.app   # 重启微信
sudo wx init
wx sessions   # 验证：能看到会话列表即正常
```

### 3. 编译 desktop-hand

```bash
cd ../desktop-hand
cargo build --release
# 产物：target/release/hand（macOS/Linux）或 hand.exe（Windows）
# config.toml 中已配置绝对路径 D:\workspace\dexa\desktop-hand\target\release\hand.exe
```

### 4. 编译 wx-agent

```bash
cd ../wechat-agent
cargo build --release
# 产物：target/release/wx-agent
```

---

## 配置

复制并编辑 `config.toml`：

```toml
[binaries]
wx   = "wx"     # wx-cli 路径，可改为绝对路径
hand = "hand"   # desktop-hand 路径，可改为绝对路径

[claude]
api_key       = "sk-ant-..."         # 或设置环境变量 WX_AGENT_API_KEY
reply_model   = "claude-haiku-4-5-20251001"
distill_model = "claude-sonnet-4-6"

[agent]
mode           = "semi"   # semi = 每条回复需确认 | auto = 全自动发送
poll_interval  = 5        # 轮询新消息的间隔（秒）
reply_max_len  = 80       # 生成回复的最大字数
require_profile = true    # 只对已蒸馏的联系人自动回复

[wechat]
# macOS 默认 activate_cmd = "open -a WeChat"，search_key = "cmd+f"
# Windows 默认 activate_cmd = "cmd /c start WeChat.exe"，search_key = "ctrl+f"
# 如需自定义，取消注释并修改：
# activate_cmd = "open -a WeChat"
# search_key   = "cmd+f"
```

**API Key 优先级**：环境变量 `WX_AGENT_API_KEY` > `config.toml` 中的 `api_key`。

---

## 使用

### 第一步：蒸馏联系人画像

在启用自动回复前，先分析想要自动回复的联系人：

```bash
wx-agent distill contact "张三"
```

输出示例：
```
Exporting messages for 「张三」…
  342 messages fetched, distilling…

=== 联系人画像：张三 ===
关系    ：close_friend
风格    ：casual
话题    ：工作、美食、旅行、电影
情感    ：积极乐观，偶尔抱怨工作压力
策略    ：保持轻松随意的语气，可以适当开玩笑
概括    ：老朋友，常聊日常和吐槽，说话直接
画像已保存到本地数据库。
```

查看所有已蒸馏的联系人：
```bash
wx-agent distill list
```

### 第二步：启动自动回复守护进程

**半自动模式**（推荐先用这个熟悉效果）：
```bash
wx-agent watch
```

收到消息时终端会显示：
```
┌─────────────────────────────────────────────
│ 来自 张三: 明天有空吗？
│ 建议回复: 有啊，什么事？
└─────────────────────────────────────────────
[Enter] 发送  [e] 编辑  [s] 跳过:
```

**全自动模式**：
```bash
wx-agent watch --auto
```

### 其他命令

```bash
# 测试 UI 自动化发送（不需要新消息触发）
wx-agent send "张三" "你好啊"

# 查看某个联系人的画像
wx-agent profile "张三"

# 自我蒸馏 → 生成 Hermes/OpenClaw SKILL.md
wx-agent distill self
wx-agent distill self --from "张三"   # 只分析与张三的对话
```

自我蒸馏的输出文件：`~/.claude/skills/wechat-self/SKILL.md`，
安装后在 Claude Code 中执行 `/wechat-self` 即可调用。

---

## 数据存储

所有分析结果本地保存，不上传：

```
~/.wx-agent/
├── config.toml     # 配置（可选位置）
└── data.db         # SQLite：联系人画像 + 消息处理记录
```

---

## 权限说明

| 平台    | 所需权限                                      |
|---------|-----------------------------------------------|
| macOS   | Accessibility 权限（首次运行 `hand` 时系统提示）|
| Windows | 管理员权限（wx-cli init 需要读取进程内存）       |

---

## 免责声明

本工具仅用于处理**自己的**微信数据，请遵守相关法律法规，不得用于未经授权的数据访问。

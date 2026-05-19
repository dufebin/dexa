---
name: wechat-agent
description: "微信自动化智能体：下载工具 → 初始化 wx-cli → 蒸馏联系人画像 → 自动监听并回复消息。仅支持 Windows（WeChat 桌面端所在机器）。"
version: 1.0.0
platforms: [windows]
metadata:
  hermes:
    tags: [wechat, 微信, auto-reply, distill, ui-automation, wx-agent]
    related_skills: []
---

# wechat-agent — 微信智能自动回复

在 Hermes 中调用此 skill（`/wechat-agent`），它会一步一步引导你在 **Windows 机器**（运行微信的那台）上完成整套部署。

---

## 系统架构

```
wx-agent.exe（调度层）
    │
    ├── wx-cli.exe          读取微信本地 SQLCipher 加密数据库
    │   sessions / history / new-messages / export
    │
    ├── head.exe            鼠标键盘 UI 自动化（发送消息）
    │   key combo / key paste / key tap
    │
    └── vision-brain.exe    LLM 蒸馏 + 回复生成（支持任意模型）
        llm distill-contact → 联系人画像 JSON
        llm generate-reply  → 候选回复文本
        llm distill-self    → 自我人格 Markdown
```

LLM 提供方通过 vision-brain 的环境变量配置（`LLM_PROVIDER` / `LLM_API_KEY` / `LLM_MODEL` / `LLM_API_URL`），支持 Anthropic、OpenAI 及所有兼容接口。

### 数据流

```
微信本地 SQLCipher DB
    ↓ wx-cli.exe 解密读取
new-messages / history / export
    ↓
wx-agent（SQLite 缓存 %USERPROFILE%\.wx-agent\data.db）
    ├─ 蒸馏: vision-brain.exe llm distill-contact → ContactProfile → data.db
    └─ 回复: vision-brain.exe llm generate-reply  → 候选回复 → head.exe → 微信 UI
```

---

## 六步完成部署

```
Step 1  下载四个二进制文件
Step 2  初始化 wx-cli（需要管理员权限 + 微信运行中）
Step 3  创建 config.toml
Step 4  设置 Claude API Key
Step 5  蒸馏联系人画像
Step 6  启动监听守护进程
```

---

## Step 1 — 下载二进制文件

在 Windows 机器上，用 **管理员身份** 打开 PowerShell，运行：

```powershell
# 创建安装目录
New-Item -ItemType Directory -Force -Path "C:\dexa"

# 下载四个工具
Invoke-WebRequest -Uri "https://fang.deephealth.net/wx-cli.exe"      -OutFile "C:\dexa\wx-cli.exe"
Invoke-WebRequest -Uri "https://fang.deephealth.net/head.exe"         -OutFile "C:\dexa\head.exe"
Invoke-WebRequest -Uri "https://fang.deephealth.net/wx-agent.exe"     -OutFile "C:\dexa\wx-agent.exe"
Invoke-WebRequest -Uri "https://fang.deephealth.net/vision-brain.exe" -OutFile "C:\dexa\vision-brain.exe"

# 验证（应看到 4 个文件）
Get-ChildItem C:\dexa\*.exe | Select-Object Name, Length
```

---

## Step 2 — 初始化 wx-cli

wx-cli 通过读取微信进程内存获取数据库解密密钥，**必须满足两个条件**：

1. 微信客户端已运行并登录
2. PowerShell 以管理员身份运行

```powershell
# 初始化（读取微信进程内存，获取数据库密钥）
C:\dexa\wx-cli.exe init

# 验证（能看到会话列表即成功）
C:\dexa\wx-cli.exe sessions
```

**成功标志**：看到 JSON 格式的会话列表（联系人名/群名 + 未读数）。

### 常见报错

| 报错 | 原因 | 解决 |
|------|------|------|
| `access denied` / 权限不足 | PowerShell 未以管理员运行 | 右键 PowerShell → 以管理员身份运行 |
| `WeChat not found` / 找不到微信进程 | 微信未运行 | 打开微信并登录后重试 |
| `sessions` 返回空数组 | init 未成功 | 重新执行 `init`，确认微信已登录 |

---

## Step 3 — 创建 config.toml

在 `C:\dexa\` 创建 `config.toml`：

```powershell
@"
[binaries]
wx   = "C:\\dexa\\wx-cli.exe"
hand = "C:\\dexa\\head.exe"

[vision_brain]
bin = "C:\\dexa\\vision-brain.exe"

[agent]
mode            = "semi"
poll_interval   = 5
reply_max_len   = 80
require_profile = true

[wechat]
activate_cmd = "C:\\dexa\\head.exe key combo --keys ctrl+q"
search_key   = "ctrl+f"
"@ | Set-Content -Encoding UTF8 "C:\dexa\config.toml"
```

> **`activate_cmd` 说明**：用 `head.exe` 发送 `Ctrl+Q` 快捷键唤出/隐藏微信窗口。如果你的微信没有绑定 `Ctrl+Q`，改为 `"cmd /c start WeChat.exe"` 或 `"cmd /c start Weixin.exe"`。

---

## Step 4 — 设置 LLM API Key

LLM 配置通过环境变量传给 `vision-brain.exe`，**不写在 config.toml 里**。

### 使用 Anthropic Claude（默认）

```powershell
# 当前会话有效
$env:LLM_PROVIDER = "anthropic"
$env:LLM_API_KEY  = "sk-ant-api03-..."
# 模型留空则使用 vision-brain 默认（claude-haiku-4-5-20251001）
```

永久设置（需要重开 PowerShell 生效）：

```powershell
[System.Environment]::SetEnvironmentVariable("LLM_PROVIDER", "anthropic", "User")
[System.Environment]::SetEnvironmentVariable("LLM_API_KEY",  "sk-ant-api03-...", "User")
```

> Anthropic API Key 在 https://console.anthropic.com/ 申请。

### 使用 OpenAI 兼容接口

```powershell
$env:LLM_PROVIDER = "openai"
$env:LLM_API_KEY  = "your-key"
$env:LLM_MODEL    = "gpt-4o-mini"      # 或其他模型名
$env:LLM_API_URL  = "https://api.openai.com/v1/chat/completions"
```

兼容接口（如 Mimo、DeepSeek 等）同理，替换 `LLM_API_URL` 和 `LLM_MODEL` 即可。

### 验证配置正常

```powershell
cd C:\dexa
.\wx-agent.exe distill list
```

输出 `暂无已蒸馏的联系人` 即表示 config.toml 读取正常，数据库可访问。

---

## Step 5 — 蒸馏联系人画像

蒸馏是**一次性操作**，分析联系人的历史消息，生成画像存入本地数据库。之后每次自动回复都会使用这份画像。

### 蒸馏指定联系人

```powershell
cd C:\dexa
.\wx-agent.exe distill contact "联系人备注名"
```

**输出示例**：
```
Exporting messages for 「张三」…
  342 messages fetched, distilling…

=== 联系人画像：张三 ===
关系    ：close_friend
风格    ：casual
话题    ：工作、美食、旅行
策略    ：保持轻松随意的语气
画像已保存到本地数据库。
```

### 查看所有已蒸馏联系人

```powershell
.\wx-agent.exe distill list
```

### 联系人名称怎么填

填写**微信备注名**，与 `wx-cli.exe sessions` 输出一致。不确定时先查：

```powershell
C:\dexa\wx-cli.exe sessions
```

---

## Step 6 — 启动监听守护进程

### 半自动模式（推荐先用）

```powershell
cd C:\dexa
.\wx-agent.exe watch
```

收到消息时终端显示：

```
┌─────────────────────────────────────────────
│ 来自 张三: 明天有空吗？
│ 建议回复: 有啊，什么事？
└─────────────────────────────────────────────
[Enter] 发送  [e] 编辑  [s] 跳过:
```

| 按键 | 动作 |
|------|------|
| Enter | 直接发送建议回复 |
| e + Enter | 手动编辑后发送 |
| s + Enter | 跳过 |
| 任意文字 + Enter | 用输入的文字作为回复 |

### 全自动模式

```powershell
.\wx-agent.exe watch --auto
```

### 测试发送（Step 6 正式开始前先跑这个）

```powershell
.\wx-agent.exe send "文件传输助手" "你好，这是测试消息"
```

---

## 数据存储

全部本地，不上传：

```
%USERPROFILE%\.wx-agent\
└── data.db     SQLite：联系人画像 + 消息处理记录
```

---

## 故障排查

| 现象 | 解决 |
|------|------|
| `wx-cli.exe` 找不到会话 | 登录微信后以管理员重跑 `init` |
| 发送到了错误联系人 | 先跑 `send 文件传输助手 "测试"` 确认链路；如果卡顿，在 config 里增大 `activate_cmd` 后的等待 |
| 中文输入乱码 | 确认 config 里 `hand` 路径指向 `head.exe`，它使用剪贴板粘贴而非模拟键盘 |
| `LLM_API_KEY env var not set` | 设置 `LLM_API_KEY` 环境变量 |
| `vision-brain llm ... failed: 401` | `LLM_API_KEY` 不正确 |
| `vision-brain llm ... failed: 429` | 增大 `poll_interval`（改为 10 或 15） |
| `wx-agent.exe` 找不到 config | `cd C:\dexa` 后再运行 |

---

## 引导策略

1. **逐步确认**：每完成一个 Step，问用户是否成功，再进入下一步。
2. **报错即介入**：用户粘贴报错，立即给出精确解法，不让用户自己猜。
3. **名称是最常见坑**：Step 5 蒸馏前，提醒用户先用 `wx-cli.exe sessions` 复制精确名称。
4. **测试优先**：Step 6 正式开 `watch` 前，先让用户跑一次 `send 文件传输助手 "测试"` 确认链路正常。

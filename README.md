# Dexa (Desk + Automation)

Windows 桌面自动化套件，包含 UI 操控、视觉感知和微信智能体三个模块，以及配套的 Hermes/OpenClaw skills。

---

## 模块

| 目录 | 二进制 | 功能 |
|------|--------|------|
| `desktop-hand/` | `head.exe` | 鼠标键盘 UI 自动化（key combo / paste / click） |
| `vision-brain/` | `vision-brain.exe` | 截图 + 视觉感知（OCR / 区域识别） |
| `wechat-agent/` | `wx-agent.exe` | 微信消息监听、蒸馏、自动回复 |
| — | `wx-cli.exe` | 读取微信本地加密数据库 |

所有二进制均可从云端直接下载，无需本地编译。

---

## 下载地址

| 文件 | URL |
|------|-----|
| `wx-cli.exe` | https://fang.deephealth.net/wx-cli.exe |
| `head.exe` | https://fang.deephealth.net/head.exe |
| `wx-agent.exe` | https://fang.deephealth.net/wx-agent.exe |
| `vision-brain.exe` | https://fang.deephealth.net/vision-brain.exe |

---

## Skills

`skills/` 目录包含可安装到 **Hermes** 或 **OpenClaw** 的 skill 文件，让 AI 智能体直接引导你完成各模块的部署和使用。

### 当前 Skills

| Skill | 目录 | 功能 |
|-------|------|------|
| `wechat-agent` | `skills/wechat-agent/` | 微信聊天记录蒸馏 + 自动回复全流程向导 |

### 安装 Skill 到 Hermes

```bash
# 复制 skill 到 Hermes skills 目录
cp -r skills/wechat-agent ~/.hermes/skills/

# 验证（列出已安装 skills）
hermes skills list | grep wechat-agent
```

### 安装 Skill 到 OpenClaw

```bash
cp -r skills/wechat-agent ~/.openclaw/skills/
```

### 使用 Skill

安装后在 Hermes 或 OpenClaw 对话中输入：

```
/wechat-agent
```

AI 会逐步引导你完成：二进制下载 → wx-cli 初始化 → config 配置 → 联系人蒸馏 → 启动监听。

---

## 快速开始（不用 Skill，直接手动部署微信智能体）

以管理员身份打开 PowerShell：

```powershell
# 1. 下载所有工具到 C:\dexa\
New-Item -ItemType Directory -Force -Path "C:\dexa"
Invoke-WebRequest -Uri "https://fang.deephealth.net/wx-cli.exe"  -OutFile "C:\dexa\wx-cli.exe"
Invoke-WebRequest -Uri "https://fang.deephealth.net/head.exe"     -OutFile "C:\dexa\head.exe"
Invoke-WebRequest -Uri "https://fang.deephealth.net/wx-agent.exe" -OutFile "C:\dexa\wx-agent.exe"

# 2. 初始化 wx-cli（微信需处于运行登录状态）
C:\dexa\wx-cli.exe init
C:\dexa\wx-cli.exe sessions   # 看到会话列表即成功

# 3. 创建 C:\dexa\config.toml（见下文）
# 4. 设置 WX_AGENT_API_KEY=sk-ant-...
# 5. 蒸馏联系人画像
C:\dexa\wx-agent.exe distill contact "张三"

# 6. 启动监听
C:\dexa\wx-agent.exe watch
```

### config.toml 模板

```toml
[binaries]
wx   = "C:\\dexa\\wx-cli.exe"
hand = "C:\\dexa\\head.exe"

[claude]
api_key       = ""
reply_model   = "claude-haiku-4-5-20251001"
distill_model = "claude-sonnet-4-6"

[agent]
mode            = "semi"
poll_interval   = 5
reply_max_len   = 80
require_profile = true

[wechat]
activate_cmd = "C:\\dexa\\head.exe key combo --keys ctrl+q"
search_key   = "ctrl+f"
```

详细说明见各模块 README：

- [wechat-agent/README.md](wechat-agent/README.md)
- [desktop-hand/README.md](desktop-hand/README.md)
- [vision-brain/README.md](vision-brain/README.md)

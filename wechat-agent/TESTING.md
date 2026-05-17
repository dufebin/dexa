# Dexa WeChat Agent — Claude Code 测试与验证指南 (TESTING.md)

本指南专为 **Claude Code (`claude` CLI)** 及自动化测试 Agent 编写，旨在提供结构化、可复现的 `wechat-agent` 及 `desktop-hand` 模块测试工作流。通过执行本指南中的步骤，Claude Code 可以快速验证编译构建、UI 自动化控制（特别是中文剪贴板直贴）、微信底层数据解包及大模型全链路的健康度。

---

## 1. 环境与前置配置检查

在执行任何功能测试前，请指引 Claude Code 检查并确认以下基础环境与配置文件：

### 1.1 检查配置文件 (`wechat-agent/config.toml`)
确保 `config.toml` 中的二进制路径正确指向本地内置与编译产物的绝对路径：
```toml
[binaries]
wx   = "D:\\workspace\\dexa\\wechat-agent\\crates\\wx-cli\\wx-cli.exe"
hand = "D:\\workspace\\dexa\\desktop-hand\\target\\release\\hand.exe"

[wechat]
activate_cmd = "D:\\workspace\\dexa\\desktop-hand\\target\\release\\hand.exe key combo --keys ctrl+q"
search_key   = "ctrl+f"
```
*注：`activate_cmd` 配置为通过 `ctrl+q` 快捷键唤醒/隐藏微信窗口，避免了寻找 `WeChat.exe` 或 `Weixin.exe` 进程的兼容性问题。*

### 1.2 验证底层微信数据访问 (`wx-cli`)
在 Windows 宿主机上（需登录微信且保持运行状态），通过 PowerShell 验证内置的 `wx-cli.exe` 是否正常工作：
```powershell
# 验证能否正常获取当前微信的会话列表
D:\workspace\dexa\wechat-agent\crates\wx-cli\wx-cli.exe sessions
```
*期望结果：输出 JSON 格式的最近会话列表及未读消息数。若提示权限不足，需以管理员身份运行。*

---

## 2. 分步测试工作流

### Step 1: 编译构建验证 (Build Verification)
验证 `desktop-hand` 与 `wechat-agent` 的 Release 构建是否能够成功通过编译。

```powershell
# 1. 编译 desktop-hand（包含 arboard 剪贴板支持）
cd D:\workspace\dexa\desktop-hand
cargo build --release

# 验证产物是否存在
Get-Item target\release\hand.exe

# 2. 编译 wechat-agent
cd D:\workspace\dexa\wechat-agent
cargo build --release

# 验证产物是否存在
Get-Item target\release\wx-agent.exe
```

---

### Step 2: 中文剪贴板 UI 自动化测试 (UI Automation & CJK Paste Test)
由于 Windows 系统输入法（IME）会拦截模拟按键导致中文输入乱码或吞字，本项目采用了**剪贴板直贴技术 (`arboard` + `Ctrl+V`)**。请依次验证单点能力与端到端发送能力。

#### 2.1 单点剪贴板直贴测试 (`desktop-hand`)
```powershell
cd D:\workspace\dexa\desktop-hand
# 测试将中文写入剪贴板并模拟发出 Ctrl+V
target\release\hand.exe key paste --text "你好 Claude Code"
```
*验证说明：执行时会观察到文本被写入当前激活的输入框内。*

#### 2.2 端到端微信消息发送测试 (`wechat-agent send`)
在真实 Windows 桌面环境下运行以下命令，测试自动搜索联系人并发送中文消息：
```powershell
cd D:\workspace\dexa\wechat-agent
target\release\wx-agent.exe send 文件传输助手 "你好，这是来自 Claude Code 的自动化测试消息！"
```
**底层执行流时序检查**：
1. **窗口激活**：调用 `hand.exe key combo --keys ctrl+q` 唤出微信窗口，等待 `1500ms`。
2. **打开搜索栏**：调用 `hand.exe key combo --keys ctrl+f`，等待 `600ms`。
3. **输入联系人**：调用 `ctrl+a` 清空旧输入，等待 `200ms`；调用 `hand.exe key paste` 写入 `文件传输助手`，等待 `800ms`。
4. **双回车选中**：连续模拟两次 `return`（间隔 `500ms`），确保绝对选中目标并进入右侧聊天窗口且锁定输入框焦点，等待 `800ms`。
5. **粘贴消息**：调用 `hand.exe key paste` 写入中文测试消息，等待 `400ms` 让微信完成文本渲染。
6. **双发机制发送**：依次模拟发送 `return` 和 `ctrl+return`（间隔 `300ms`），完美覆盖微信设置为 `Enter` 或 `Ctrl+Enter` 发送的两种情况。

---

### Step 3: 底层数据解包与本人账号识别测试 (Data Extraction & is_self Test)
验证 `wx-agent` 与底层 `wx-cli.exe` 的通信解包能力，以及通过智能查询 `文件传输助手` 识别用户本人账号（`is_self`）的后置处理逻辑。

```powershell
cd D:\workspace\dexa\wechat-agent
# 执行联系人画像蒸馏前置的数据导出步骤
target\release\wx-agent.exe distill contact "文件传输助手"
```
**期望结果与验证点**：
1. 控制台打印：`Exporting messages for 「文件传输助手」…` 以及 `17 messages fetched, distilling…`。
2. **结构自适应解包成功**：完美兼容 `wx export` 返回的包装对象结构 `{ "display": "...", "messages": [...], "username": "..." }`。
3. **本人账号判别成功**：程序自动识别当前登录账号昵称，成功区分 `contact_msgs` 与 `self_msgs`。
4. *注：若未配置 API Key，在此步最后请求 LLM 时提示 `401 Unauthorized` 属于完全正常的预期行为，证明本地数据获取与解析链路已 100% 畅通。*

---

### Step 4: 大模型画像蒸馏与全自动守护测试 (LLM & Watch Loop Test)
当需要完整验证大模型画像生成与自动回复守护线程时，请按以下指引进行配置：

#### 4.1 配置 API Key
在 PowerShell 中设置临时环境变量（或直接修改 `config.toml` 中的 `[claude]` 部分）：
```powershell
$env:WX_AGENT_API_KEY="your_actual_api_key_here"
```

#### 4.2 执行联系人画像蒸馏 (`distill`)
```powershell
cd D:\workspace\dexa\wechat-agent
target\release\wx-agent.exe distill contact "文件传输助手"
```
*期望结果：调用大模型分析聊天记录成功，并在 SQLite 数据库 `~/.wx-agent/data.db` 的 `contact_profiles` 表中生成并保存结构化 JSON 档案。*

#### 4.3 执行半自动/全自动消息监听守护 (`watch`)
```powershell
cd D:\workspace\dexa\wechat-agent
# 启动监听守护线程
target\release\wx-agent.exe watch
```
*期望结果：程序每 `5` 秒轮询一次 `wx-cli.exe new-messages`，发现新消息后自动拉取上下文与联系人画像，生成建议回复并在半自动模式下提示用户按 `[Enter]` 发送。*

---

## 3. 常见错误排查指南 (Troubleshooting)

| 错误现象 | 可能原因 | 解决/排查方案 |
| :--- | :--- | :--- |
| **`hand: command not found` 或找不到 hand.exe** | 未配置绝对路径或未加入 PATH | 检查 `config.toml` 中 `hand` 参数是否为绝对路径 `D:\...\hand.exe`。 |
| **消息发送时输入了乱码或拼音字母** | 系统输入法 (IME) 拦截了模拟按键 | 确认 `wechat_ui.rs` 中已使用 `key_paste` 代替 `key_type`，且系统剪贴板未被其他软件独占。 |
| **微信弹出后未输入文字或输入到错误窗口** | 微信弹出速度慢于 `1500ms` 缓冲时间 | 检查微信是否已登录并常驻后台。可在 `wechat_ui.rs` 中适当增大 `activate_wechat` 后的 `sleep` 时间。 |
| **`wx sessions` 报错或无返回** | 微信未运行或 PowerShell 权限不足 | 确保微信客户端正在运行并已登录；以管理员身份重新运行 PowerShell 终端。 |
| **`Database::open` 提示无法打开数据库文件** | SQLite 数据目录 `~/.wx-agent` 不存在 | 此问题已在 `db.rs` 中修复（自动创建父级目录）。若依然报错，请检查用户目录读写权限。 |

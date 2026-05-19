---
name: wx-send
description: 通过微信 UI 自动化向指定联系人发送消息
metadata:
  type: tool
  binary: wx-agent
---

## 功能

使用 `desktop-hand` 模拟键鼠操作，在微信客户端中搜索联系人并发送消息。支持中文、Emoji 等 Unicode 内容（通过剪贴板粘贴）。

## 使用方法

```bash
wx-agent send <联系人名> <消息内容>
```

示例：
```bash
wx-agent send "张三" "你好，在吗？"
wx-agent send "李四" "刚才的文件已经发给你了"
```

联系人名需与微信中显示的备注名或昵称一致（用于搜索框搜索）。

## 发送步骤（自动执行）

1. 激活微信窗口
2. 打开搜索框（macOS: `Cmd+F`，Windows: `Ctrl+F`）
3. 输入联系人名并选中
4. 粘贴消息内容并按回车发送

## 配置（config.toml）

```toml
[binaries]
hand = "hand"              # desktop-hand 二进制路径

[wechat]
search_key   = "ctrl+f"   # 搜索快捷键（Windows），macOS 默认 cmd+f
activate_cmd = ""          # 激活微信的命令（可选）
```

## 在 Hermes / OpenClaw 中

激活后可用自然语言指令，例如：
- "帮我发消息给张三：明天下午三点见"
- "给李四发一条：好的收到"

AI 将提取联系人和消息内容，执行 `wx-agent send` 命令。

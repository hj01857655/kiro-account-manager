# Local Kiro 2API Server 功能文档

## 功能概述

Local Kiro 2API Server 是集成在 Kiro Account Manager 中的本地 API 转换服务。它将 OpenAI 兼容的 API 请求转换为 Anthropic Claude API 调用，让你可以在任何支持 OpenAI API 的工具中使用 Claude 模型。

## 特性

- ✅ **一键启动** - 在应用内直接启动/停止服务器
- ✅ **自动认证** - 自动使用账号管理器中的 token
- ✅ **实时日志** - 查看服务器运行日志
- ✅ **本地运行** - 无需外部部署，数据安全
- ✅ **OpenAI 兼容** - 支持 `/v1/chat/completions` 等标准端点

## 使用方法

### 1. 启动服务器

1. 打开 Kiro Account Manager
2. 点击左侧菜单 **"Local Server"**
3. 点击 **"Start Server"** 按钮
4. 等待日志显示服务器启动成功

### 2. 访问管理面板

浏览器打开：`http://127.0.0.1:7860`

### 3. 配置 API 客户端

在支持 OpenAI API 的工具中配置：

```
Base URL: http://127.0.0.1:7860/v1
API Key: local-sidecar-token
```

## API 端点

| 端点 | 方法 | 描述 |
|------|------|------|
| `/` | GET | Web 管理界面 |
| `/api/tokens` | GET | Token 池状态 |
| `/v1/models` | GET | 可用模型列表 |
| `/v1/chat/completions` | POST | 聊天补全 (OpenAI 格式) |
| `/v1/messages` | POST | 消息 API (Anthropic 格式) |

## 支持的模型

- `claude-sonnet-4-5`
- `claude-sonnet-4-5-20250929`
- `claude-sonnet-4-20250514`
- `claude-3-7-sonnet-20250219`
- `claude-3-5-haiku-20241022`
- `claude-haiku-4-5-20251001`

## 示例请求

### 聊天补全

```bash
curl -X POST http://127.0.0.1:7860/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer local-sidecar-token" \
  -d '{
    "model": "claude-sonnet-4-5",
    "messages": [{"role": "user", "content": "Hello!"}],
    "max_tokens": 100
  }'
```

### 获取模型列表

```bash
curl http://127.0.0.1:7860/v1/models \
  -H "Authorization: Bearer local-sidecar-token"
```

### 查看 Token 状态

```bash
curl http://127.0.0.1:7860/api/tokens
```

## 在常用工具中配置

### Cursor

1. 打开 Cursor 设置
2. 找到 "OpenAI API" 配置
3. 设置 Base URL: `http://127.0.0.1:7860/v1`
4. 设置 API Key: `local-sidecar-token`

### Continue (VS Code 插件)

在 `~/.continue/config.json` 中添加：

```json
{
  "models": [{
    "title": "Claude (Local)",
    "provider": "openai",
    "model": "claude-sonnet-4-5",
    "apiBase": "http://127.0.0.1:7860/v1",
    "apiKey": "local-sidecar-token"
  }]
}
```

### OpenAI Python SDK

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://127.0.0.1:7860/v1",
    api_key="local-sidecar-token"
)

response = client.chat.completions.create(
    model="claude-sonnet-4-5",
    messages=[{"role": "user", "content": "Hello!"}]
)
print(response.choices[0].message.content)
```

## 故障排除

### 服务器启动失败

1. **没有可用账号** - 请先在账号管理器中添加账号
2. **端口被占用** - 检查 7860 端口是否被其他程序占用
3. **二进制文件缺失** - 重新编译服务器二进制

### Token 刷新失败

- 部分 token 显示 401 错误是正常的（token 过期）
- 只要有至少一个 token 刷新成功，服务器就能正常工作

### 无法访问管理面板

- 确保服务器已启动（状态显示 "RUNNING"）
- 使用 `http://127.0.0.1:7860` 而不是 `localhost:7860`

## 技术架构

```
┌─────────────────────────────────────────────────────────┐
│                 Kiro Account Manager                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │   账号管理   │  │  Local Server │  │   设置/关于    │  │
│  │   (React)   │  │    (React)   │  │    (React)     │  │
│  └─────────────┘  └──────┬──────┘  └─────────────────┘  │
│                          │                               │
│  ┌───────────────────────┴───────────────────────────┐  │
│  │              Tauri Backend (Rust)                  │  │
│  │  - 账号存储                                        │  │
│  │  - 进程管理 (server_process.rs)                   │  │
│  │  - Token 注入                                      │  │
│  └───────────────────────┬───────────────────────────┘  │
└──────────────────────────┼──────────────────────────────┘
                           │
                           ▼
              ┌────────────────────────┐
              │   kiro-server Binary   │
              │   (Deno Compiled)      │
              │                        │
              │  - OpenAI API 转换     │
              │  - Token 池管理        │
              │  - 请求负载均衡        │
              └───────────┬────────────┘
                          │
                          ▼
              ┌────────────────────────┐
              │   Anthropic Claude API │
              └────────────────────────┘
```

## 开发者信息

### 重新编译服务器

```bash
cd kiro-account-manager
./scripts/build-server.sh
```

### 相关文件

- `server/` - kiro2api-deno 源码
- `src-tauri/src/server_process.rs` - Rust 进程管理
- `src/contexts/LocalServerContext.jsx` - React 状态管理
- `src/components/LocalServer/` - UI 组件
- `src-tauri/bin/kiro-server-*` - 编译后的二进制

### 环境变量

服务器启动时自动注入以下环境变量：

| 变量 | 描述 |
|------|------|
| `PORT` | 服务器端口 (默认 7860) |
| `KIRO_KV_PATH` | KV 数据库路径 |
| `KIRO_CLIENT_TOKEN` | API 认证 token |
| `KIRO_AUTH_TOKEN` | 账号 token 列表 (JSON) |

## 更新日志

### v1.0.0 (2025-12-10)
- ✅ 初始版本
- ✅ 基础启动/停止功能
- ✅ 实时日志显示
- ✅ 自动 token 注入
- ✅ Web 管理面板

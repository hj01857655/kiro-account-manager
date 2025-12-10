# Quick Start Guide - kiro2api Deno

## 最快速启动（30秒）

```bash
# 1. 进入目录
cd deno-impl

# 2. 设置环境变量
export KIRO_CLIENT_TOKEN="123456"
export KIRO_AUTH_TOKEN='[{"auth":"Social","refreshToken":"your_token_here"}]'

# 3. 运行
deno task start
```

## 测试 API

服务启动后，在新终端运行：

```bash
# 测试健康检查
curl http://localhost:8080/

# 测试模型列表
curl -H "Authorization: Bearer 123456" \
  http://localhost:8080/v1/models

# 测试聊天（需要有效的 AWS token）
curl -X POST http://localhost:8080/v1/messages \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer 123456" \
  -d '{
    "model": "claude-sonnet-4-20250514",
    "max_tokens": 100,
    "messages": [
      {"role": "user", "content": "你好"}
    ]
  }'
```

## 使用 .env 文件

```bash
# 1. 复制配置文件
cp .env.example .env

# 2. 编辑 .env 文件
nano .env  # 或使用你喜欢的编辑器

# 3. 运行
deno task start
```

## Docker 快速启动

```bash
# 1. 构建并启动
docker-compose up -d

# 2. 查看日志
docker logs -f kiro2api-deno

# 3. 停止
docker-compose down
```

## 常见问题

### Q: 如何获取 AWS token？

A: 查看主项目 README.md 中的"Token获取方式"部分

### Q: 端口被占用怎么办？

```bash
PORT=8081 deno task start
```

### Q: 如何启用调试日志？

```bash
LOG_LEVEL=debug deno task start
```

### Q: 需要安装什么依赖吗？

A: 不需要！Deno 会自动下载所需的一切

## 性能测试

```bash
# 简单压测（需要安装 wrk）
wrk -t4 -c100 -d10s \
  -H "Authorization: Bearer 123456" \
  http://localhost:8080/v1/models
```

## 开发模式

```bash
# 带自动重载的开发模式
deno task dev

# 修改代码后会自动重启
```

## 下一步

详细文档请参阅：

- [完整 README](./README.md)
- [主项目文档](../README.md)
- [Deno 文档](https://deno.land/manual)

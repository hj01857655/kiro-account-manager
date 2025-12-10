# 部署指南 - kiro2api Deno

## Deno Deploy 部署（推荐）

Deno Deploy 是最简单的云部署方式，提供全球边缘计算节点。

### 方法 1: 使用 GitHub 集成（推荐）

1. **准备代码仓库**
   ```bash
   # 将 deno-impl 目录推送到 GitHub
   git add deno-impl/
   git commit -m "Add Deno implementation"
   git push origin main
   ```

2. **创建 Deno Deploy 项目**
   - 访问 [dash.deno.com](https://dash.deno.com)
   - 点击 "New Project"
   - 连接你的 GitHub 仓库
   - 选择 `deno-impl/main.ts` 作为入口点

3. **配置环境变量** 在 Deno Deploy 项目设置中添加：
   ```
   KIRO_CLIENT_TOKEN=your-secure-token
   KIRO_AUTH_TOKEN=[{"auth":"Social","refreshToken":"your_token"}]
   ```

4. **部署**
   - 保存配置后会自动部署
   - 获得类似 `https://your-project.deno.dev` 的 URL

### 方法 2: 使用 deployctl CLI

```bash
# 安装 deployctl
deno install -A --global https://deno.land/x/deploy/deployctl.ts

# 登录
deployctl login

# 部署
deployctl deploy \
  --project=kiro2api \
  --entrypoint=main.ts \
  --env=KIRO_CLIENT_TOKEN=your-token \
  --env=KIRO_AUTH_TOKEN='[...]'
```

### 方法 3: 使用 GitHub Actions

创建 `.github/workflows/deploy.yml`:

```yaml
name: Deploy to Deno Deploy

on:
  push:
    branches: [main]
    paths:
      - "deno-impl/**"

jobs:
  deploy:
    runs-on: ubuntu-latest
    permissions:
      id-token: write
      contents: read

    steps:
      - uses: actions/checkout@v3

      - uses: denoland/setup-deno@v1
        with:
          deno-version: v2.x

      - name: Deploy to Deno Deploy
        uses: denoland/deployctl@v1
        with:
          project: kiro2api
          entrypoint: deno-impl/main.ts
          root: deno-impl
        env:
          DENO_DEPLOY_TOKEN: ${{ secrets.DENO_DEPLOY_TOKEN }}
```

在 GitHub 仓库设置中添加 Secrets：

- `DENO_DEPLOY_TOKEN`: 从 Deno Deploy 获取
- `KIRO_CLIENT_TOKEN`: 你的 API token
- `KIRO_AUTH_TOKEN`: AWS 认证配置

---

## Docker 部署

### Docker Compose（推荐）

```bash
cd deno-impl

# 编辑 .env 文件
cp .env.example .env
nano .env

# 启动
docker-compose up -d

# 查看日志
docker logs -f kiro2api-deno

# 停止
docker-compose down
```

### 单独 Docker 命令

```bash
# 构建
docker build -t kiro2api-deno .

# 运行
docker run -d \
  --name kiro2api-deno \
  -p 8080:8080 \
  -e KIRO_CLIENT_TOKEN="your-token" \
  -e KIRO_AUTH_TOKEN='[{"auth":"Social","refreshToken":"your_token"}]' \
  kiro2api-deno

# 查看日志
docker logs -f kiro2api-deno
```

---

## VPS 部署（systemd）

### 1. 安装 Deno

```bash
curl -fsSL https://deno.land/install.sh | sh
echo 'export PATH="$HOME/.deno/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### 2. 部署应用

```bash
# 克隆代码
git clone <your-repo>
cd kiro2api/deno-impl

# 配置环境变量
cp .env.example .env
nano .env

# 测试运行
deno task start
```

### 3. 创建 systemd 服务

创建 `/etc/systemd/system/kiro2api.service`:

```ini
[Unit]
Description=kiro2api Deno Service
After=network.target

[Service]
Type=simple
User=your-user
WorkingDirectory=/path/to/kiro2api/deno-impl
EnvironmentFile=/path/to/kiro2api/deno-impl/.env
ExecStart=/home/your-user/.deno/bin/deno run --allow-net --allow-env --allow-read main.ts
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

```bash
# 重载配置
sudo systemctl daemon-reload

# 启动服务
sudo systemctl start kiro2api

# 开机自启
sudo systemctl enable kiro2api

# 查看状态
sudo systemctl status kiro2api

# 查看日志
sudo journalctl -u kiro2api -f
```

---

## Nginx 反向代理

```nginx
server {
    listen 80;
    server_name api.yourdomain.com;

    location / {
        proxy_pass http://localhost:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

```bash
# 测试配置
sudo nginx -t

# 重载 Nginx
sudo systemctl reload nginx
```

---

## 监控和日志

### 健康检查

```bash
# 基础健康检查
curl https://your-domain.com/

# 检查模型列表
curl -H "Authorization: Bearer your-token" \
  https://your-domain.com/v1/models

# 检查 Token 池状态
curl https://your-domain.com/api/tokens
```

### 性能监控

使用 Deno Deploy 内置的监控面板，或者集成：

- **Sentry**: 错误追踪
- **DataDog**: APM 监控
- **Prometheus**: 指标收集

---

## 故障排除

### 1. 环境变量未设置

**错误**: `KIRO_CLIENT_TOKEN environment variable not set`

**解决**:

- Deno Deploy: 在项目设置中添加环境变量
- Docker: 检查 .env 文件或 docker-compose.yml
- VPS: 检查 systemd EnvironmentFile 路径

### 2. AWS Token 刷新失败

**错误**: `Token refresh failed`

**解决**:

- 检查 KIRO_AUTH_TOKEN 格式是否正确
- 确认 AWS token 未过期
- 验证网络连接到 AWS 服务

### 3. 权限错误

**本地开发错误**: `PermissionDenied`

**解决**:

```bash
# 确保授予所需权限
deno run --allow-net --allow-env --allow-read main.ts
```

### 4. 端口被占用

```bash
# 更改端口
PORT=8081 deno task start
```

---

## 安全建议

1. **使用强密码**: `KIRO_CLIENT_TOKEN` 应该是随机生成的强密码
2. **HTTPS**: 生产环境必须使用 HTTPS
3. **限流**: 考虑添加 rate limiting
4. **监控**: 设置告警监控异常行为
5. **定期更新**: 保持 Deno 和依赖项最新

---

## 性能优化

### Deno Deploy

- 自动全球 CDN 分发
- 边缘计算节点
- 自动扩展

### Docker

```yaml
# docker-compose.yml 优化
services:
  kiro2api-deno:
    deploy:
      resources:
        limits:
          cpus: "1"
          memory: 512M
        reservations:
          cpus: "0.5"
          memory: 256M
```

### VPS

```bash
# 使用 PM2 管理进程（备选方案）
npm install -g pm2
pm2 start "deno run --allow-net --allow-env --allow-read main.ts" --name kiro2api
pm2 save
pm2 startup
```

---

## 下一步

部署成功后：

1. 测试 API 端点
2. 配置监控和告警
3. 设置自动备份
4. 文档化你的部署流程

更多信息：

- [Deno Deploy 文档](https://deno.com/deploy/docs)
- [Docker 文档](https://docs.docker.com/)
- [主项目 README](../README.md)

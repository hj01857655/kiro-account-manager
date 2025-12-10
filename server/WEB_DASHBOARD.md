# Web Dashboard 使用指南

## 概述

kiro2api-deno 现在包含了一个现代化的 Web 管理界面，用于实时监控 Token 池的状态和使用情况。

## 访问方式

启动服务器后，在浏览器中访问：

```
http://localhost:8080/
```

或者如果部署到服务器：

```
http://your-server-address:port/
```

## 功能特性

### 1. 实时状态概览

顶部状态栏显示：
- **总 Token 数**: 配置的 Token 总数量
- **可用 Token**: 当前可用的 Token 数量
- **最后更新**: 最近一次刷新的时间

### 2. Token 详细信息表格

显示每个 Token 的详细信息：
- **用户邮箱**: Token 关联的用户邮箱
- **Token 预览**: Token 的前几位字符（脱敏显示）
- **认证方式**: Social 或 IdC
- **剩余次数**: 该 Token 的剩余可用次数
- **过期时间**: Token 的过期时间
- **最后使用**: 最近一次使用该 Token 的时间
- **状态**: Token 的当前状态（正常/即将耗尽/已耗尽/已过期）

### 3. 状态标识

Token 状态用不同颜色的徽章表示：

- 🟢 **正常** (绿色): Token 工作正常，剩余次数充足
- 🟠 **即将耗尽** (橙色): 剩余次数 ≤ 5
- ⚫ **已耗尽** (灰色): 剩余次数为 0
- 🔴 **已过期** (红色): Token 已过期

### 4. 控制功能

#### 手动刷新
点击「🔄 手动刷新」按钮可以立即刷新 Token 状态。

#### 自动刷新
点击「自动刷新」开关可以启用/禁用自动刷新功能。启用后，界面会每 30 秒自动刷新一次数据。

## 设计特点

- 🎨 **现代化界面**: 渐变色背景、毛玻璃效果
- 📱 **响应式设计**: 适配桌面和移动设备
- ⚡ **实时更新**: 无需刷新页面即可查看最新状态
- 🔒 **无需认证**: Dashboard 访问无需 API Token

## API 端点

Web Dashboard 使用以下 API 端点获取数据：

```
GET /api/tokens
```

该端点返回 JSON 格式的 Token 池状态信息。您也可以直接调用此端点来集成到其他系统中。

### 响应示例

```json
{
  "total_tokens": 2,
  "active_tokens": 1,
  "tokens": [
    {
      "user_email": "user@example.com",
      "token_preview": "arn:aws:sso...***",
      "auth_type": "Social",
      "remaining_usage": 100,
      "expires_at": "2025-10-20T12:00:00Z",
      "last_used": "2025-10-19T10:30:00Z"
    }
  ]
}
```

## 故障排除

### 界面无法加载

1. 确认服务器已正常启动
2. 检查浏览器控制台是否有错误信息
3. 确认 `/static/` 目录存在且包含所需文件

### Token 数据不显示

1. 检查 `KIRO_AUTH_TOKEN` 环境变量是否正确配置
2. 查看服务器日志确认 Token 刷新是否成功
3. 尝试手动访问 `/api/tokens` 端点

### 样式显示异常

1. 清除浏览器缓存
2. 确认 CSS 文件正确加载（检查浏览器开发者工具）
3. 尝试硬刷新页面（Ctrl+Shift+R 或 Cmd+Shift+R）

## 技术细节

### 文件结构

```
static/
├── index.html          # 主页面
├── css/
│   └── dashboard.css   # 样式表
└── js/
    └── dashboard.js    # 前端逻辑
```

### 技术栈

- **前端**: 纯 JavaScript (ES6+)
- **样式**: CSS3 (Flexbox, Grid, Animations)
- **设计模式**: 面向对象、单一职责原则

### 浏览器兼容性

- Chrome/Edge: ✅ 完全支持
- Firefox: ✅ 完全支持
- Safari: ✅ 完全支持
- IE11: ❌ 不支持

## 未来计划

- [ ] 添加 Token 使用趋势图表
- [ ] 支持 Token 手动刷新
- [ ] 添加告警通知功能
- [ ] 支持深色模式切换
- [ ] 添加筛选和排序功能

## 反馈

如有问题或建议，请在 GitHub 上提交 Issue。

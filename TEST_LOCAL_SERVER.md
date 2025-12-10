# 🧪 Local Server 测试指南

## ✅ 已完成
- ✅ Deno 已安装
- ✅ 服务器二进制已编译 (76MB)
- ✅ npm 依赖已安装

## 🚀 启动应用

```bash
cd /Users/macmima1234/Desktop/kiro项目/kiro-account-manager
npm run tauri dev
```

## 📋 测试步骤

### 1. 基础功能测试
1. 应用启动后，点击左侧 Sidebar 的 **"Local Server"** 菜单
2. 查看服务器状态（应该显示 "Stopped" 和红色指示灯）
3. 点击 **"Start Server"** 按钮
4. 观察：
   - 状态变为 "Running"，绿色指示灯
   - 日志区域开始显示服务器输出
   - 端口显示为 7860

### 2. 日志测试
- 查看日志输出是否正常显示
- 尝试点击 "Clear" 按钮清空日志
- 日志应该自动滚动到底部

### 3. 服务器访问测试
打开浏览器访问：
```
http://127.0.0.1:7860
```
应该看到 kiro2api 的管理界面

### 4. API 测试
在终端运行：
```bash
# 测试模型列表
curl http://127.0.0.1:7860/v1/models

# 测试 token 状态
curl http://127.0.0.1:7860/api/tokens
```

### 5. 停止测试
- 点击 **"Stop Server"** 按钮
- 状态应该变为 "Stopped"，红色指示灯
- 浏览器访问应该失败

## ⚠️ 已知限制

### 当前使用占位符 Token
服务器使用的是占位符认证 token：`local-sidecar-token-placeholder`

如果需要真实的 API 功能，需要：
1. 配置 Anthropic API Key
2. 修改 `src-tauri/src/server_process.rs` 注入真实的 API keys

### 端口配置
- 当前端口固定为 7860
- ConfigPanel UI 已创建但未连接到后端
- 需要实现端口配置持久化

## 🐛 可能的问题

### 问题 1: 二进制文件未找到
**错误**: "Binary not found" 或 "Failed to spawn server"

**解决**:
```bash
ls -la src-tauri/bin/
# 应该看到 kiro-server-x86_64-apple-darwin
```

### 问题 2: 端口被占用
**错误**: 服务器启动失败，日志显示端口占用

**解决**:
```bash
# 查找占用 7860 端口的进程
lsof -i :7860
# 杀死进程
kill -9 <PID>
```

### 问题 3: 权限问题
**错误**: Permission denied

**解决**:
```bash
chmod +x src-tauri/bin/kiro-server-x86_64-apple-darwin
```

## 📊 成功标准

- ✅ 服务器能成功启动
- ✅ 日志实时显示
- ✅ 状态指示器正确更新
- ✅ 能访问 http://127.0.0.1:7860
- ✅ 服务器能正常停止

## 🎯 下一步优化

1. **API Key 注入**: 从账号管理器获取真实的 Anthropic API keys
2. **端口配置**: 实现动态端口配置和持久化
3. **自动启动**: 添加应用启动时自动启动服务器选项
4. **错误处理**: 优化错误提示和恢复机制
5. **性能监控**: 添加 CPU/内存使用显示

## 📝 测试报告模板

```
测试日期: ____
测试人: ____

[ ] 应用启动成功
[ ] Local Server 菜单可见
[ ] 服务器启动成功
[ ] 日志正常显示
[ ] 浏览器可访问
[ ] API 响应正常
[ ] 服务器停止成功

问题记录:
1. ____
2. ____

建议:
1. ____
2. ____
```

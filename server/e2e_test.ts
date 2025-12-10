#!/usr/bin/env -S deno run --allow-net --allow-env --allow-read --allow-write --unstable-kv
/**
 * 端到端测试 - 验证 API 端点
 * 
 * 运行: deno run --allow-net --allow-env --allow-read --allow-write --unstable-kv e2e_test.ts
 * 
 * 注意：此测试需要设置环境变量：
 * - KIRO_CLIENT_TOKEN: API 认证密钥
 * - KIRO_AUTH_TOKEN: AWS 认证配置 (JSON)
 */

const PORT = 18080; // 使用测试端口避免冲突
const BASE_URL = `http://localhost:${PORT}`;
const TEST_TOKEN = "test-token-12345";

let testsPassed = 0;
let testsFailed = 0;
let serverProcess: Deno.ChildProcess | null = null;

function log(message: string) {
  console.log(`[E2E] ${message}`);
}

function assert(condition: boolean, message: string) {
  if (condition) {
    console.log(`✅ PASS: ${message}`);
    testsPassed++;
  } else {
    console.error(`❌ FAIL: ${message}`);
    testsFailed++;
  }
}

// 启动测试服务器
async function startTestServer(): Promise<boolean> {
  log("启动测试服务器...");
  
  // 设置测试环境变量
  const env = {
    PORT: PORT.toString(),
    KIRO_CLIENT_TOKEN: TEST_TOKEN,
    KIRO_AUTH_TOKEN: JSON.stringify([
      {
        auth: "Social",
        refreshToken: "test-token-placeholder"
      }
    ]),
    LOG_LEVEL: "error", // 减少日志输出
  };

  try {
    const command = new Deno.Command("deno", {
      args: ["task", "start"],
      env,
      stdout: "null",
      stderr: "null",
    });

    serverProcess = command.spawn();
    
    // 等待服务器启动 (最多 5 秒)
    for (let i = 0; i < 10; i++) {
      await new Promise(resolve => setTimeout(resolve, 500));
      
      try {
        const response = await fetch(`${BASE_URL}/`, {
          method: "GET",
          signal: AbortSignal.timeout(1000),
        });
        
        if (response.status === 200) {
          log("✓ 服务器已启动");
          return true;
        }
      } catch {
        // 继续等待
      }
    }
    
    log("✗ 服务器启动超时");
    return false;
  } catch (error) {
    log(`✗ 启动服务器失败: ${error}`);
    return false;
  }
}

// 停止测试服务器
async function stopTestServer() {
  if (serverProcess) {
    log("停止测试服务器...");
    try {
      serverProcess.kill("SIGTERM");
      await serverProcess.status;
      log("✓ 服务器已停止");
    } catch {
      serverProcess.kill("SIGKILL");
    }
    serverProcess = null;
  }
}

// 测试健康检查端点
async function testHealthCheck() {
  log("测试健康检查端点...");
  
  try {
    const response = await fetch(`${BASE_URL}/`, {
      method: "GET",
      signal: AbortSignal.timeout(3000),
    });
    
    assert(response.status === 200, "健康检查返回 200");
    assert(response.headers.get("content-type")?.includes("text/html") === true, "返回 HTML 内容");
  } catch (error) {
    console.error(`健康检查失败: ${error}`);
    testsFailed++;
  }
}

// 测试模型列表端点
async function testModelsEndpoint() {
  log("测试模型列表端点...");
  
  try {
    const response = await fetch(`${BASE_URL}/v1/models`, {
      method: "GET",
      headers: {
        "Authorization": `Bearer ${TEST_TOKEN}`,
      },
      signal: AbortSignal.timeout(3000),
    });
    
    assert(response.status === 200, "模型列表返回 200");
    
    const data = await response.json();
    assert(data.object === "list", "响应对象类型正确");
    assert(Array.isArray(data.data), "包含模型数据数组");
    assert(data.data.length > 0, "至少有一个模型");
    
    // 检查模型格式
    const firstModel = data.data[0];
    assert(firstModel.id !== undefined, "模型有 id");
    assert(firstModel.object === "model", "模型对象类型正确");
  } catch (error) {
    console.error(`模型列表测试失败: ${error}`);
    testsFailed++;
  }
}

// 测试无效认证
async function testInvalidAuth() {
  log("测试无效认证...");
  
  try {
    const response = await fetch(`${BASE_URL}/v1/models`, {
      method: "GET",
      headers: {
        "Authorization": "Bearer invalid-token",
      },
      signal: AbortSignal.timeout(3000),
    });
    
    assert(response.status === 401 || response.status === 403, "无效 token 返回 401/403");
  } catch (error) {
    console.error(`无效认证测试失败: ${error}`);
    testsFailed++;
  }
}

// 测试 CORS headers
async function testCORSHeaders() {
  log("测试 CORS headers...");
  
  try {
    const response = await fetch(`${BASE_URL}/v1/models`, {
      method: "OPTIONS",
      headers: {
        "Origin": "https://example.com",
      },
      signal: AbortSignal.timeout(3000),
    });
    
    const corsHeader = response.headers.get("access-control-allow-origin");
    assert(corsHeader === "*" || corsHeader !== null, "CORS headers 已设置");
  } catch (error) {
    console.error(`CORS 测试失败: ${error}`);
    testsFailed++;
  }
}

// 测试错误格式的请求
async function testInvalidRequest() {
  log("测试错误格式的请求...");
  
  try {
    const response = await fetch(`${BASE_URL}/v1/messages`, {
      method: "POST",
      headers: {
        "Authorization": `Bearer ${TEST_TOKEN}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        // 缺少必需字段
        messages: []
      }),
      signal: AbortSignal.timeout(3000),
    });
    
    assert(response.status >= 400, "错误请求返回 4xx 状态码");
  } catch (error) {
    console.error(`无效请求测试失败: ${error}`);
    testsFailed++;
  }
}

// 主测试流程
async function runTests() {
  console.log("🧪 开始端到端测试...\n");
  
  // 启动服务器
  const serverStarted = await startTestServer();
  
  if (!serverStarted) {
    console.error("\n❌ 无法启动服务器，跳过测试");
    Deno.exit(1);
  }
  
  try {
    // 等待服务器稳定
    await new Promise(resolve => setTimeout(resolve, 1000));
    
    // 运行测试
    await testHealthCheck();
    await testModelsEndpoint();
    await testInvalidAuth();
    await testCORSHeaders();
    await testInvalidRequest();
    
  } finally {
    // 停止服务器
    await stopTestServer();
  }
  
  // 测试总结
  console.log("\n" + "═".repeat(60));
  console.log("测试完成！");
  console.log(`✅ 通过: ${testsPassed}`);
  console.log(`❌ 失败: ${testsFailed}`);
  console.log(`📊 总计: ${testsPassed + testsFailed}`);
  console.log("═".repeat(60));
  
  if (testsFailed > 0) {
    console.log("\n❌ 部分测试失败");
    Deno.exit(1);
  } else {
    console.log("\n🎉 所有端到端测试通过！");
    Deno.exit(0);
  }
}

// 运行测试
runTests().catch((error) => {
  console.error("测试运行失败:", error);
  stopTestServer();
  Deno.exit(1);
});

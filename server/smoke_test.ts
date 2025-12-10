#!/usr/bin/env -S deno run --allow-net --allow-env --allow-read
/**
 * 冒烟测试 - 验证核心功能
 * 
 * 运行: deno run --allow-net --allow-env --allow-read smoke_test.ts
 */

import { openAIToAnthropic } from "./converter/converter.ts";
import { convertAnthropicToOpenAI } from "./converter/openai.ts";

let testsPassed = 0;
let testsFailed = 0;

function assert(condition: boolean, message: string) {
  if (condition) {
    console.log(`✅ PASS: ${message}`);
    testsPassed++;
  } else {
    console.error(`❌ FAIL: ${message}`);
    testsFailed++;
  }
}

function assertEquals(actual: unknown, expected: unknown, message: string) {
  const passed = JSON.stringify(actual) === JSON.stringify(expected);
  assert(passed, message);
  if (!passed) {
    console.error("  Expected:", expected);
    console.error("  Actual:", actual);
  }
}

console.log("🧪 开始冒烟测试...\n");

// ============================================================
// 测试1: OpenAI 到 Anthropic 格式转换
// ============================================================
console.log("📋 测试 OpenAI → Anthropic 转换");

const openAIRequest = {
  model: "claude-sonnet-4-20250514",
  messages: [
    { role: "user", content: "你好" }
  ],
  max_tokens: 1000,
  temperature: 0.7,
  stream: false
};

try {
  const anthropicReq = openAIToAnthropic(openAIRequest);
  
  assert(anthropicReq.model !== undefined, "转换后的请求包含 model 字段");
  assert(anthropicReq.messages.length === 1, "转换后的消息数量正确");
  assertEquals(anthropicReq.messages[0].role, "user", "消息角色正确");
  assert(anthropicReq.max_tokens === 1000, "max_tokens 转换正确");
  
  console.log();
} catch (error) {
  console.error("❌ OpenAI → Anthropic 转换失败:", error);
  testsFailed++;
}

// ============================================================
// 测试2: Anthropic 到 OpenAI 格式转换
// ============================================================
console.log("📋 测试 Anthropic → OpenAI 转换");

const anthropicResponse = {
  id: "msg_123",
  type: "message",
  role: "assistant",
  content: [
    {
      type: "text",
      text: "你好！我是 Claude。"
    }
  ],
  model: "claude-sonnet-4-20250514",
  stop_reason: "end_turn",
  usage: {
    input_tokens: 10,
    output_tokens: 20
  }
};

try {
  const openAIResp = convertAnthropicToOpenAI(anthropicResponse, openAIRequest.model, "chatcmpl-123");
  
  assert(openAIResp.id !== undefined, "转换后的响应包含 id");
  assert(openAIResp.choices.length === 1, "转换后包含 1 个 choice");
  assert(openAIResp.choices[0].message.role === "assistant", "角色转换正确");
  assert(openAIResp.choices[0].message.content === "你好！我是 Claude。", "内容转换正确");
  assert(openAIResp.usage?.prompt_tokens === 10, "input_tokens 转换正确");
  assert(openAIResp.usage?.completion_tokens === 20, "output_tokens 转换正确");
  
  console.log();
} catch (error) {
  console.error("❌ Anthropic → OpenAI 转换失败:", error);
  testsFailed++;
}

// ============================================================
// 测试3: 工具调用格式转换
// ============================================================
console.log("📋 测试工具调用格式转换");

const openAIWithTools = {
  model: "claude-sonnet-4-20250514",
  messages: [
    { role: "user", content: "今天天气如何？" }
  ],
  tools: [
    {
      type: "function",
      function: {
        name: "get_weather",
        description: "获取天气信息",
        parameters: {
          type: "object",
          properties: {
            location: { type: "string", description: "城市名称" }
          },
          required: ["location"]
        }
      }
    }
  ]
};

try {
  const anthropicWithTools = openAIToAnthropic(openAIWithTools);
  
  assert(anthropicWithTools.tools !== undefined, "工具列表已转换");
  assert(anthropicWithTools.tools!.length === 1, "工具数量正确");
  assertEquals(anthropicWithTools.tools![0].name, "get_weather", "工具名称正确");
  
  console.log();
} catch (error) {
  console.error("❌ 工具调用转换失败:", error);
  testsFailed++;
}

// ============================================================
// 测试4: 图片内容处理
// ============================================================
console.log("📋 测试图片内容处理");

const openAIWithImage = {
  model: "claude-sonnet-4-20250514",
  messages: [
    {
      role: "user",
      content: [
        { type: "text", text: "这是什么？" },
        {
          type: "image_url",
          image_url: {
            url: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
          }
        }
      ]
    }
  ]
};

try {
  const anthropicWithImage = openAIToAnthropic(openAIWithImage);
  
  assert(Array.isArray(anthropicWithImage.messages[0].content), "内容是数组格式");
  const content = anthropicWithImage.messages[0].content as Array<{type: string}>;
  assert(content.some(c => c.type === "image"), "包含图片内容块");
  
  console.log();
} catch (error) {
  console.error("❌ 图片内容处理失败:", error);
  testsFailed++;
}

// ============================================================
// 测试5: 模型映射
// ============================================================
console.log("📋 测试模型映射");

import { MODEL_MAP } from "./config/constants.ts";

try {
  assert(MODEL_MAP["claude-sonnet-4-20250514"] !== undefined, "claude-sonnet-4-20250514 有映射");
  assert(MODEL_MAP["claude-3-5-haiku-20241022"] !== undefined, "claude-3-5-haiku-20241022 有映射");
  assertEquals(MODEL_MAP["claude-3-5-haiku-20241022"], "auto", "Haiku 模型映射为 auto");
  
  console.log();
} catch (error) {
  console.error("❌ 模型映射测试失败:", error);
  testsFailed++;
}

// ============================================================
// 测试总结
// ============================================================
console.log("═".repeat(60));
console.log(`测试完成！`);
console.log(`✅ 通过: ${testsPassed}`);
console.log(`❌ 失败: ${testsFailed}`);
console.log(`📊 总计: ${testsPassed + testsFailed}`);
console.log("═".repeat(60));

if (testsFailed > 0) {
  Deno.exit(1);
} else {
  console.log("\n🎉 所有测试通过！");
  Deno.exit(0);
}

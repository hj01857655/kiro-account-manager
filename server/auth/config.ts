import { KVStore } from "./kv_store.ts";

// Authentication configuration types
export interface AuthConfig {
  auth: "Social" | "IdC";
  refreshToken: string;
  clientId?: string;
  clientSecret?: string;
  disabled?: boolean;
  description?: string;
}

// Load authentication configurations from KV or environment
export async function loadAuthConfigs(): Promise<AuthConfig[]> {
  // First, try to load from KV
  try {
    const kvStore = await KVStore.create();
    const kvConfigs = await kvStore.getAuthConfigs();
    kvStore.close();
    
    if (kvConfigs && kvConfigs.length > 0) {
      console.log(`✅ 从 KV 加载了 ${kvConfigs.length} 个认证配置`);
      // Filter and validate
      const enabledConfigs = kvConfigs.filter((c) => !c.disabled);
      if (enabledConfigs.length > 0) {
        return validateConfigs(enabledConfigs);
      }
    }
  } catch (error) {
    console.warn(`⚠️  从 KV 加载配置失败，将使用环境变量:`, error);
  }
  
  // Fallback to environment variable
  return await loadAuthConfigsFromEnv();
}

// Load authentication configurations from environment
async function loadAuthConfigsFromEnv(): Promise<AuthConfig[]> {
  // Check for deprecated environment variables
  const deprecatedVars = [
    "REFRESH_TOKEN",
    "AWS_REFRESHTOKEN",
    "IDC_REFRESH_TOKEN",
    "BULK_REFRESH_TOKENS",
  ];

  for (const envVar of deprecatedVars) {
    if (Deno.env.get(envVar)) {
      console.warn(`⚠️  检测到已弃用的环境变量: ${envVar}`);
      console.warn(`   请迁移到 KIRO_AUTH_TOKEN 的 JSON 格式`);
      console.warn(`   示例: KIRO_AUTH_TOKEN='[{"auth":"Social","refreshToken":"your_token"}]'`);
    }
  }

  const authToken = Deno.env.get("KIRO_AUTH_TOKEN");

  if (!authToken) {
    throw new Error(
      "未找到 KIRO_AUTH_TOKEN 环境变量\n" +
      "请设置: KIRO_AUTH_TOKEN='[{\"auth\":\"Social\",\"refreshToken\":\"your_token\"}]'\n" +
      "或设置为配置文件路径: KIRO_AUTH_TOKEN=/path/to/config.json\n" +
      "支持的认证方式: Social, IdC\n" +
      "详细配置请参考: .env.example"
    );
  }

  // Check if it's a file path
  try {
    const stat = await Deno.stat(authToken);
    if (stat.isFile) {
      const content = await Deno.readTextFile(authToken);
      return parseAuthConfigs(content);
    }
  } catch {
    // Not a file, treat as JSON string
  }

  // Parse as JSON string
  return parseAuthConfigs(authToken);
}

function validateConfigs(configs: AuthConfig[]): AuthConfig[] {
  // Validate configs
  for (const config of configs) {
    if (!config.auth || !config.refreshToken) {
      throw new Error("Invalid auth config: missing auth or refreshToken");
    }

    if (config.auth === "IdC") {
      if (!config.clientId || !config.clientSecret) {
        throw new Error("IdC auth requires clientId and clientSecret");
      }
    }
  }
  return configs;
}

function parseAuthConfigs(jsonString: string): AuthConfig[] {
  try {
    const configs = JSON.parse(jsonString) as AuthConfig[];

    if (!Array.isArray(configs)) {
      throw new Error("KIRO_AUTH_TOKEN must be a JSON array");
    }

    // Filter out disabled configs
    const enabledConfigs = configs.filter((c) => !c.disabled);

    if (enabledConfigs.length === 0) {
      throw new Error("No enabled authentication configs found");
    }

    return validateConfigs(enabledConfigs);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Failed to parse auth configs: ${message}`);
  }
}

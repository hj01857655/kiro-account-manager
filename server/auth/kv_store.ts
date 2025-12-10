import type { AuthConfig } from "./config.ts";
import * as logger from "../logger/logger.ts";

const KV_KEY = "kiro_auth_tokens";

export class KVStore {
  private kv: Deno.Kv;

  private constructor(kv: Deno.Kv) {
    this.kv = kv;
  }

  static async create(): Promise<KVStore> {
    const kvPath = Deno.env.get("KIRO_KV_PATH");
    if (kvPath) {
      logger.info(`Opening KV store at custom path: ${kvPath}`);
    }
    const kv = kvPath ? await Deno.openKv(kvPath) : await Deno.openKv();
    return new KVStore(kv);
  }

  /**
   * 获取所有存储的 auth configs
   */
  async getAuthConfigs(): Promise<AuthConfig[] | null> {
    try {
      const result = await this.kv.get<AuthConfig[]>([KV_KEY]);
      return result.value;
    } catch (error) {
      logger.error("Failed to get auth configs from KV", logger.Err(error));
      return null;
    }
  }

  /**
   * 保存 auth configs 到 KV
   */
  async saveAuthConfigs(configs: AuthConfig[]): Promise<boolean> {
    try {
      await this.kv.set([KV_KEY], configs);
      logger.info(`已保存 ${configs.length} 个 auth configs 到 KV`);
      return true;
    } catch (error) {
      logger.error("Failed to save auth configs to KV", logger.Err(error));
      return false;
    }
  }

  /**
   * 添加单个 auth config
   */
  async addAuthConfig(config: AuthConfig): Promise<boolean> {
    try {
      const configs = await this.getAuthConfigs() || [];
      configs.push(config);
      return await this.saveAuthConfigs(configs);
    } catch (error) {
      logger.error("Failed to add auth config", logger.Err(error));
      return false;
    }
  }

  /**
   * 删除指定的 auth config (通过 refreshToken)
   */
  async deleteAuthConfig(refreshToken: string): Promise<boolean> {
    try {
      const configs = await this.getAuthConfigs() || [];
      const filtered = configs.filter(c => c.refreshToken !== refreshToken);
      
      if (filtered.length === configs.length) {
        logger.warn("未找到要删除的 token");
        return false;
      }
      
      return await this.saveAuthConfigs(filtered);
    } catch (error) {
      logger.error("Failed to delete auth config", logger.Err(error));
      return false;
    }
  }

  /**
   * 批量导入 auth configs (覆盖现有的)
   */
  async importAuthConfigs(configs: AuthConfig[]): Promise<boolean> {
    try {
      return await this.saveAuthConfigs(configs);
    } catch (error) {
      logger.error("Failed to import auth configs", logger.Err(error));
      return false;
    }
  }

  /**
   * 清空所有 auth configs
   */
  async clearAuthConfigs(): Promise<boolean> {
    try {
      await this.kv.delete([KV_KEY]);
      logger.info("已清空所有 auth configs");
      return true;
    } catch (error) {
      logger.error("Failed to clear auth configs", logger.Err(error));
      return false;
    }
  }

  /**
   * 关闭 KV 连接
   */
  close() {
    this.kv.close();
  }
}

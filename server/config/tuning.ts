// 性能和行为调优参数
// 从硬编码提取为可配置常量，遵循 KISS 原则

// ========== 解析器配置 ==========

// 解析器容忍的最大错误次数，用于所有解析器，防止死循环
export const PARSER_MAX_ERRORS = 5;

// ========== Token缓存配置 ==========

// Token缓存的生存时间（毫秒），过期后需要重新刷新
export const TOKEN_CACHE_TTL = 5 * 60 * 1000; // 5分钟

// HTTP客户端Keep-Alive间隔（毫秒）
export const HTTP_CLIENT_KEEP_ALIVE = 30 * 1000; // 30秒

// HTTP客户端TLS握手超时（毫秒）
export const HTTP_CLIENT_TLS_HANDSHAKE_TIMEOUT = 15 * 1000; // 15秒

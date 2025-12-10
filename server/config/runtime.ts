// Token管理常量
export const TOKEN_CACHE_KEY_FORMAT = "token_%d";
export const TOKEN_REFRESH_CLEANUP_DELAY = 5000; // 5秒（毫秒）
// 与Go版本一致的Token缓存TTL（5分钟，毫秒）
export const TOKEN_CACHE_TTL_MS = 5 * 60 * 1000;

// 消息处理常量
export const MESSAGE_ID_FORMAT = "msg_%s";
export const MESSAGE_ID_TIME_FORMAT = "yyyyMMddHHmmss";
export const RETRY_DELAY = 100; // 100毫秒

// Token估算常量
export const BASE_TOOLS_OVERHEAD = 100;
export const SHORT_TEXT_THRESHOLD = 100;
export const LONG_TEXT_THRESHOLD = 1000;

// EventStream解析器常量
export const EVENT_STREAM_MIN_MESSAGE_SIZE = 16;
export const EVENT_STREAM_MAX_MESSAGE_SIZE = 16 * 1024 * 1024; // 16MB

// Token计算常量
export const TOKEN_ESTIMATION_RATIO = 4;

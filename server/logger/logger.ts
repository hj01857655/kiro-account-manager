/**
 * Logger Module - Structured logging with levels and file output
 * Based on kiro2api Go version logger implementation
 */

export enum Level {
  DEBUG = 0,
  INFO = 1,
  WARN = 2,
  ERROR = 3,
  FATAL = 4,
}

const levelNames: Record<Level, string> = {
  [Level.DEBUG]: "DEBUG",
  [Level.INFO]: "INFO",
  [Level.WARN]: "WARN",
  [Level.ERROR]: "ERROR",
  [Level.FATAL]: "FATAL",
};

interface Field {
  key: string;
  value: unknown | (() => unknown);
}

interface LogEntry {
  timestamp: string;
  level: string;
  file?: string;
  func?: string;
  message: string;
  [key: string]: unknown;
}

class Logger {
  private level: Level = Level.INFO;
  private logFile: Deno.FsFile | null = null;
  private enableConsole = true;
  private enableCaller = false;
  private format: "json" | "text" = "json";

  constructor() {
    this.initialize();
  }

  private initialize() {
    // Set level from environment
    const logLevel = Deno.env.get("LOG_LEVEL");
    if (logLevel) {
      const parsedLevel = this.parseLevel(logLevel);
      if (parsedLevel !== null) {
        this.level = parsedLevel;
      }
    }

    // Enable caller info in debug mode
    if (this.level === Level.DEBUG) {
      this.enableCaller = true;
    }

    // Set log format
    const logFormat = Deno.env.get("LOG_FORMAT");
    if (logFormat === "text" || logFormat === "json") {
      this.format = logFormat;
    }

    // Set console output
    const logConsole = Deno.env.get("LOG_CONSOLE");
    if (logConsole === "false") {
      this.enableConsole = false;
    }

    // Open log file if specified
    const logFilePath = Deno.env.get("LOG_FILE");
    if (logFilePath) {
      try {
        this.logFile = Deno.openSync(logFilePath, {
          create: true,
          write: true,
          append: true,
        });
      } catch (error) {
        console.error(`Failed to open log file ${logFilePath}:`, error);
      }
    }
  }

  private parseLevel(s: string): Level | null {
    switch (s.toUpperCase().trim()) {
      case "DEBUG":
        return Level.DEBUG;
      case "INFO":
        return Level.INFO;
      case "WARN":
      case "WARNING":
        return Level.WARN;
      case "ERROR":
        return Level.ERROR;
      case "FATAL":
        return Level.FATAL;
      default:
        return null;
    }
  }

  private shouldLog(level: Level): boolean {
    return this.level <= level;
  }

  private getCallerInfo(): { file?: string; func?: string } {
    if (!this.enableCaller) {
      return {};
    }

    const stack = new Error().stack;
    if (!stack) return {};

    // Parse stack trace to get caller info
    const lines = stack.split("\n");
    // Skip first 4 lines: Error, getCallerInfo, log, and the wrapper (debug/info/warn/error)
    const callerLine = lines[4];
    if (!callerLine) return {};

    const match = callerLine.match(/at\s+(.+?)\s+\((.+?):(\d+):(\d+)\)/);
    if (match) {
      const [, funcName, filePath, line] = match;
      const fileName = filePath.split("/").pop();
      return {
        file: `${fileName}:${line}`,
        func: funcName,
      };
    }

    return {};
  }

  private log(level: Level, msg: string, fields: Field[]) {
    if (!this.shouldLog(level)) {
      return;
    }

    const entry: LogEntry = {
      timestamp: new Date().toISOString(),
      level: levelNames[level],
      message: msg,
    };

    // Add caller info if enabled
    if (this.enableCaller) {
      const caller = this.getCallerInfo();
      if (caller.file) entry.file = caller.file;
      if (caller.func) entry.func = caller.func;
    }

    // Add custom fields, skip system fields
    // 使用懒加载：只有在需要时才计算字段值
    const systemFields = new Set([
      "level",
      "log_level",
      "timestamp",
      "message",
      "file",
      "log_file",
      "func",
    ]);

    for (const field of fields) {
      if (!systemFields.has(field.key)) {
        // 懒加载：如果 value 是函数，则调用它获取实际值
        entry[field.key] = typeof field.value === "function" 
          ? (field.value as () => unknown)() 
          : field.value;
      }
    }

    // Format and output
    const output = this.format === "json" ? JSON.stringify(entry) : this.formatText(entry);

    // Write to console
    if (this.enableConsole) {
      console.log(output);
    }

    // Write to file
    if (this.logFile) {
      try {
        const encoder = new TextEncoder();
        this.logFile.writeSync(encoder.encode(output + "\n"));
      } catch (error) {
        console.error("Failed to write to log file:", error);
      }
    }

    // Exit on fatal
    if (level === Level.FATAL) {
      this.close();
      Deno.exit(1);
    }
  }

  private formatText(entry: LogEntry): string {
    const parts = [
      entry.timestamp,
      entry.level.padEnd(5),
    ];

    if (entry.file) {
      parts.push(`[${entry.file}]`);
    }

    if (entry.func) {
      parts.push(`{${entry.func}}`);
    }

    parts.push(entry.message);

    // Add extra fields
    const extraFields: string[] = [];
    for (const [key, value] of Object.entries(entry)) {
      if (
        !["timestamp", "level", "message", "file", "func"].includes(key)
      ) {
        extraFields.push(`${key}=${JSON.stringify(value)}`);
      }
    }

    if (extraFields.length > 0) {
      parts.push(`(${extraFields.join(", ")})`);
    }

    return parts.join(" ");
  }

  public debug(msg: string, ...fields: Field[]) {
    this.log(Level.DEBUG, msg, fields);
  }

  public info(msg: string, ...fields: Field[]) {
    this.log(Level.INFO, msg, fields);
  }

  public warn(msg: string, ...fields: Field[]) {
    this.log(Level.WARN, msg, fields);
  }

  public error(msg: string, ...fields: Field[]) {
    this.log(Level.ERROR, msg, fields);
  }

  public fatal(msg: string, ...fields: Field[]) {
    this.log(Level.FATAL, msg, fields);
  }

  public close() {
    if (this.logFile) {
      this.logFile.close();
      this.logFile = null;
    }
  }

  public setLevel(level: Level) {
    this.level = level;
  }
}

// Field constructors
export function String(key: string, val: string): Field {
  return { key, value: val };
}

export function Int(key: string, val: number): Field {
  return { key, value: val };
}

export function Float(key: string, val: number): Field {
  return { key, value: val };
}

export function Bool(key: string, val: boolean): Field {
  return { key, value: val };
}

export function Err(err: Error | unknown): Field {
  if (err === null || err === undefined) {
    return { key: "error", value: null };
  }
  if (err instanceof Error) {
    return {
      key: "error",
      value: {
        message: err.message,
        name: err.name,
        stack: err.stack,
      },
    };
  }
  return { key: "error", value: globalThis.String(err) };
}

export function Duration(key: string, val: number): Field {
  return { key, value: `${val}ms` };
}

export function Any(key: string, val: unknown): Field {
  return { key, value: val };
}

/**
 * 懒加载字段构造器
 * 只有在日志级别满足时才会计算字段值
 * 适用于性能敏感的计算，如 JSON.stringify、复杂对象序列化等
 */

/**
 * 懒加载字符串字段
 */
export function LazyString(key: string, fn: () => string): Field {
  return { key, value: fn };
}

/**
 * 懒加载数字字段
 */
export function LazyInt(key: string, fn: () => number): Field {
  return { key, value: fn };
}

/**
 * 懒加载任意类型字段
 */
export function LazyAny(key: string, fn: () => unknown): Field {
  return { key, value: fn };
}

/**
 * 懒加载 JSON 序列化
 * 适用于复杂对象，避免在不需要时进行序列化
 */
export function LazyJson(key: string, obj: unknown): Field {
  return { key, value: () => JSON.stringify(obj) };
}

/**
 * HTTP 状态码字段
 */
export function HttpStatus(code: number): Field {
  return { key: "http_status", value: code };
}

/**
 * 错误类型分类
 */
export function ErrorType(type: string): Field {
  return { key: "error_type", value: type };
}

/**
 * 用户标识（脱敏）
 */
export function UserId(id: string): Field {
  // 只保留前4位和后4位
  if (id.length <= 8) {
    return { key: "user_id", value: "****" };
  }
  return { key: "user_id", value: `${id.slice(0, 4)}****${id.slice(-4)}` };
}

/**
 * 延迟时间（毫秒）
 */
export function Latency(ms: number): Field {
  return { key: "latency_ms", value: ms };
}

/**
 * 字节大小
 */
export function Bytes(size: number): Field {
  return { key: "bytes", value: size };
}

/**
 * 操作阶段
 */
export function Phase(phase: string): Field {
  return { key: "phase", value: phase };
}

/**
 * 重试次数
 */
export function RetryCount(count: number): Field {
  return { key: "retry_count", value: count };
}

// Global default logger
const defaultLogger = new Logger();

// Global logger functions
export function debug(msg: string, ...fields: Field[]) {
  defaultLogger.debug(msg, ...fields);
}

export function info(msg: string, ...fields: Field[]) {
  defaultLogger.info(msg, ...fields);
}

export function warn(msg: string, ...fields: Field[]) {
  defaultLogger.warn(msg, ...fields);
}

export function error(msg: string, ...fields: Field[]) {
  defaultLogger.error(msg, ...fields);
}

export function fatal(msg: string, ...fields: Field[]) {
  defaultLogger.fatal(msg, ...fields);
}

export function setLevel(level: Level) {
  defaultLogger.setLevel(level);
}

export function close() {
  defaultLogger.close();
}

// Reinitialize logger (useful after env vars are loaded)
export function reinitialize() {
  close();
  Object.assign(defaultLogger, new Logger());
}

export { Logger };

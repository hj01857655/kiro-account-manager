// AWS Event Stream 头部解析器 - 支持断点续传和智能容错
// 参考 Go 版本的 header_parser.go 实现

export enum ParsePhase {
  ReadNameLength = 0,
  ReadName = 1,
  ReadValueType = 2,
  ReadValueLength = 3,
  ReadValue = 4,
}

export enum ValueType {
  BOOL_TRUE = 0,
  BOOL_FALSE = 1,
  BYTE = 2,
  SHORT = 3,
  INTEGER = 4,
  LONG = 5,
  BYTE_ARRAY = 6,
  STRING = 7,
  TIMESTAMP = 8,
  UUID = 9,
}

export interface HeaderValue {
  type: ValueType;
  value: unknown;
}

export interface HeaderParseState {
  phase: ParsePhase;
  currentHeader: number;
  nameLength: number;
  valueType: ValueType;
  valueLength: number;
  partialName: Uint8Array;
  partialValue: Uint8Array;
  partialLength: Uint8Array;
  parsedHeaders: Record<string, HeaderValue>;
}

export class HeaderParser {
  private state: HeaderParseState;

  constructor() {
    this.state = this.createNewState();
  }

  private createNewState(): HeaderParseState {
    return {
      phase: ParsePhase.ReadNameLength,
      currentHeader: 0,
      nameLength: 0,
      valueType: ValueType.STRING,
      valueLength: 0,
      partialName: new Uint8Array(0),
      partialValue: new Uint8Array(0),
      partialLength: new Uint8Array(0),
      parsedHeaders: {},
    };
  }

  reset(): void {
    this.state = this.createNewState();
  }

  getState(): HeaderParseState {
    return this.state;
  }

  parseHeaders(data: Uint8Array): Record<string, HeaderValue> {
    if (data.length === 0) {
      return {};
    }

    return this.parseHeadersWithState(data, this.state);
  }

  parseHeadersWithState(data: Uint8Array, state: HeaderParseState): Record<string, HeaderValue> {
    if (data.length === 0) {
      if (Object.keys(state.parsedHeaders).length > 0) {
        return state.parsedHeaders;
      }
      return {};
    }

    let offset = 0;
    let consecutiveDataInsufficientCount = 0;
    const initialPhase = state.phase;

    while (offset < data.length) {
      const currentPhase = state.phase;
      let needMoreData = false;
      let error: Error | null = null;

      switch (state.phase) {
        case ParsePhase.ReadNameLength:
          ({ needMoreData, error } = this.processNameLengthPhase(data, offset, state));
          if (!needMoreData && !error) offset++;
          break;

        case ParsePhase.ReadName:
          ({ needMoreData, error, offset } = this.processNamePhase(data, offset, state));
          break;

        case ParsePhase.ReadValueType:
          ({ needMoreData, error } = this.processValueTypePhase(data, offset, state));
          if (!needMoreData && !error) offset++;
          break;

        case ParsePhase.ReadValueLength:
          ({ needMoreData, error, offset } = this.processValueLengthPhase(data, offset, state));
          break;

        case ParsePhase.ReadValue:
          ({ needMoreData, error, offset } = this.processValuePhase(data, offset, state));
          break;
      }

      if (error) {
        console.warn(`Header parsing error: ${error.message}`);
        return state.parsedHeaders;
      }

      if (needMoreData) {
        consecutiveDataInsufficientCount++;

        // 检测循环：同一阶段连续失败
        if (currentPhase === initialPhase && consecutiveDataInsufficientCount > 2) {
          console.warn(`Parsing loop detected, forcing completion`);
          return this.forceCompleteHeaderParsing(state);
        }

        // 有部分结果但连续失败
        if (Object.keys(state.parsedHeaders).length > 0 && consecutiveDataInsufficientCount > 3) {
          console.warn(`Too many consecutive failures, forcing completion`);
          return this.forceCompleteHeaderParsing(state);
        }

        // 长期无法解析
        if (Object.keys(state.parsedHeaders).length === 0 && consecutiveDataInsufficientCount > 5) {
          console.warn(`Unable to parse any headers, using defaults`);
          return this.forceCompleteHeaderParsing(state);
        }

        // 需要更多数据
        return state.parsedHeaders;
      }

      consecutiveDataInsufficientCount = 0;
    }

    // 数据结束但解析未完成
    if (state.phase !== ParsePhase.ReadNameLength) {
      if (Object.keys(state.parsedHeaders).length > 0) {
        console.debug(`Incomplete data but have parsed headers, forcing completion`);
        return this.forceCompleteHeaderParsing(state);
      }
      // 返回部分结果
      return state.parsedHeaders;
    }

    return state.parsedHeaders;
  }

  private processNameLengthPhase(
    data: Uint8Array,
    offset: number,
    state: HeaderParseState
  ): { needMoreData: boolean; error: Error | null } {
    if (offset >= data.length) {
      return { needMoreData: true, error: null };
    }

    const nameLength = data[offset];

    if (nameLength === 0 || nameLength > 255) {
      return { needMoreData: false, error: new Error(`Invalid name length: ${nameLength}`) };
    }

    state.nameLength = nameLength;
    state.partialName = new Uint8Array(0);
    state.phase = ParsePhase.ReadName;

    return { needMoreData: false, error: null };
  }

  private processNamePhase(
    data: Uint8Array,
    offset: number,
    state: HeaderParseState
  ): { needMoreData: boolean; error: Error | null; offset: number } {
    const remainingNameBytes = state.nameLength - state.partialName.length;
    const availableBytes = data.length - offset;

    if (availableBytes < remainingNameBytes) {
      // 累积部分数据
      const partial = data.slice(offset);
      state.partialName = this.concatBuffers(state.partialName, partial);
      return { needMoreData: true, error: null, offset: data.length };
    }

    // 读取剩余数据
    const remaining = data.slice(offset, offset + remainingNameBytes);
    state.partialName = this.concatBuffers(state.partialName, remaining);
    state.phase = ParsePhase.ReadValueType;

    return { needMoreData: false, error: null, offset: offset + remainingNameBytes };
  }

  private processValueTypePhase(
    data: Uint8Array,
    offset: number,
    state: HeaderParseState
  ): { needMoreData: boolean; error: Error | null } {
    if (offset >= data.length) {
      return { needMoreData: true, error: null };
    }

    state.valueType = data[offset] as ValueType;
    state.phase = ParsePhase.ReadValueLength;

    return { needMoreData: false, error: null };
  }

  private processValueLengthPhase(
    data: Uint8Array,
    offset: number,
    state: HeaderParseState
  ): { needMoreData: boolean; error: Error | null; offset: number } {
    const remainingBytes = 2 - state.partialLength.length;
    const availableBytes = data.length - offset;

    if (availableBytes < remainingBytes) {
      // 累积部分长度数据
      const partial = data.slice(offset);
      state.partialLength = this.concatBuffers(state.partialLength, partial);
      return { needMoreData: true, error: null, offset: data.length };
    }

    // 读取剩余长度字节
    const remaining = data.slice(offset, offset + remainingBytes);
    state.partialLength = this.concatBuffers(state.partialLength, remaining);

    // 解析长度
    const view = new DataView(state.partialLength.buffer, state.partialLength.byteOffset);
    const valueLength = view.getUint16(0, false); // Big-endian

    if (valueLength < 0 || valueLength > 65535) {
      return { needMoreData: false, error: new Error(`Invalid value length: ${valueLength}`), offset: offset + remainingBytes };
    }

    state.valueLength = valueLength;
    state.partialValue = new Uint8Array(0);
    state.partialLength = new Uint8Array(0);
    state.phase = ParsePhase.ReadValue;

    return { needMoreData: false, error: null, offset: offset + remainingBytes };
  }

  private processValuePhase(
    data: Uint8Array,
    offset: number,
    state: HeaderParseState
  ): { needMoreData: boolean; error: Error | null; offset: number } {
    const remainingValueBytes = state.valueLength - state.partialValue.length;
    const availableBytes = data.length - offset;

    if (availableBytes < remainingValueBytes) {
      // 累积部分值数据
      const partial = data.slice(offset);
      state.partialValue = this.concatBuffers(state.partialValue, partial);
      return { needMoreData: true, error: null, offset: data.length };
    }

    // 读取剩余值数据
    const remaining = data.slice(offset, offset + remainingValueBytes);
    state.partialValue = this.concatBuffers(state.partialValue, remaining);

    // 解析值
    try {
      const value = this.parseHeaderValue(state.valueType, state.partialValue);
      const headerName = new TextDecoder().decode(state.partialName);
      state.parsedHeaders[headerName] = {
        type: state.valueType,
        value,
      };
    } catch (err) {
      console.warn(`Failed to parse header value:`, err);
    }

    // 重置状态，准备下一个头部
    state.currentHeader++;
    state.nameLength = 0;
    state.valueType = ValueType.STRING;
    state.valueLength = 0;
    state.partialName = new Uint8Array(0);
    state.partialValue = new Uint8Array(0);
    state.partialLength = new Uint8Array(0);
    state.phase = ParsePhase.ReadNameLength;

    return { needMoreData: false, error: null, offset: offset + remainingValueBytes };
  }

  private parseHeaderValue(valueType: ValueType, data: Uint8Array): unknown {
    const view = new DataView(data.buffer, data.byteOffset);

    switch (valueType) {
      case ValueType.BOOL_TRUE:
        return true;
      case ValueType.BOOL_FALSE:
        return false;
      case ValueType.BYTE:
        if (data.length !== 1) throw new Error(`BYTE type length error`);
        return view.getInt8(0);
      case ValueType.SHORT:
        if (data.length !== 2) throw new Error(`SHORT type length error`);
        return view.getInt16(0, false);
      case ValueType.INTEGER:
        if (data.length !== 4) throw new Error(`INTEGER type length error`);
        return view.getInt32(0, false);
      case ValueType.LONG:
        if (data.length !== 8) throw new Error(`LONG type length error`);
        return Number(view.getBigInt64(0, false));
      case ValueType.BYTE_ARRAY:
        return new Uint8Array(data);
      case ValueType.STRING:
        return new TextDecoder().decode(data);
      case ValueType.TIMESTAMP:
        if (data.length !== 8) throw new Error(`TIMESTAMP type length error`);
        return Number(view.getBigInt64(0, false));
      case ValueType.UUID:
        if (data.length === 16) {
          const hex = Array.from(data).map(b => b.toString(16).padStart(2, '0')).join('');
          return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
        }
        return new TextDecoder().decode(data);
      default:
        return new Uint8Array(data);
    }
  }

  isHeaderParseRecoverable(state: HeaderParseState): boolean {
    // 在读取名称长度阶段且已有解析成功的头部
    if (state.phase === ParsePhase.ReadNameLength && Object.keys(state.parsedHeaders).length >= 1) {
      return true;
    }
    // 在名称或值读取阶段但有基本头部信息
    return Object.keys(state.parsedHeaders).length > 0;
  }

  forceCompleteHeaderParsing(state: HeaderParseState): Record<string, HeaderValue> {
    if (Object.keys(state.parsedHeaders).length === 0) {
      // 返回默认头部
      return {
        ":message-type": { type: ValueType.STRING, value: "event" },
        ":event-type": { type: ValueType.STRING, value: "assistantResponseEvent" },
        ":content-type": { type: ValueType.STRING, value: "application/json" },
      };
    }

    // 补充缺失的关键头部
    const result = { ...state.parsedHeaders };

    if (!result[":message-type"]) {
      result[":message-type"] = { type: ValueType.STRING, value: "event" };
    }
    if (!result[":event-type"]) {
      result[":event-type"] = { type: ValueType.STRING, value: "assistantResponseEvent" };
    }
    if (!result[":content-type"]) {
      result[":content-type"] = { type: ValueType.STRING, value: "application/json" };
    }

    return result;
  }

  private concatBuffers(a: Uint8Array, b: Uint8Array): Uint8Array {
    const result = new Uint8Array(a.length + b.length);
    result.set(a, 0);
    result.set(b, a.length);
    return result;
  }
}

// 辅助函数
export function getMessageTypeFromHeaders(headers: Record<string, HeaderValue>): string {
  return (headers[":message-type"]?.value as string) || "event";
}

export function getEventTypeFromHeaders(headers: Record<string, HeaderValue>): string {
  return (headers[":event-type"]?.value as string) || "";
}

export function getContentTypeFromHeaders(headers: Record<string, HeaderValue>): string {
  return (headers[":content-type"]?.value as string) || "application/json";
}

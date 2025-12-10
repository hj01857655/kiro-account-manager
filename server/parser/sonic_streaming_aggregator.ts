type ToolParamsUpdateCallback = (toolUseId: string, fullParams: string) => void;

interface SonicJSONStreamer {
  toolUseId: string;
  toolName: string;
  buffer: string;
  lastUpdate: Date;
  isComplete: boolean;
  result: Record<string, unknown> | null;
  fragmentCount: number;
  totalBytes: number;
  incompleteUTF8: string;
  hasValidJSON: boolean;
}

export class SonicStreamingJSONAggregator {
  private activeStreamers: Map<string, SonicJSONStreamer> = new Map();
  private updateCallback?: ToolParamsUpdateCallback;

  constructor(callback?: ToolParamsUpdateCallback) {
    this.updateCallback = callback;
  }

  processToolData(
    toolUseId: string,
    name: string,
    input: string,
    stop: boolean,
    _fragmentIndex: number
  ): { complete: boolean; fullInput: string } {
    let streamer = this.activeStreamers.get(toolUseId);
    
    if (!streamer) {
      streamer = this.createStreamer(toolUseId, name);
      this.activeStreamers.set(toolUseId, streamer);
    }

    if (input) {
      this.appendFragment(streamer, input);
    }

    if (!stop) {
      return { complete: false, fullInput: "" };
    }

    this.tryParse(streamer);
    streamer.isComplete = true;

    let fullInput = "{}";
    if (streamer.hasValidJSON && streamer.result) {
      fullInput = JSON.stringify(streamer.result);
    }

    this.activeStreamers.delete(toolUseId);
    this.onAggregationComplete(toolUseId, fullInput);

    return { complete: true, fullInput };
  }

  private createStreamer(toolUseId: string, toolName: string): SonicJSONStreamer {
    return {
      toolUseId,
      toolName,
      buffer: "",
      lastUpdate: new Date(),
      isComplete: false,
      result: null,
      fragmentCount: 0,
      totalBytes: 0,
      incompleteUTF8: "",
      hasValidJSON: false,
    };
  }

  private appendFragment(streamer: SonicJSONStreamer, fragment: string): void {
    const safeFragment = this.ensureUTF8Integrity(streamer, fragment);
    streamer.buffer += safeFragment;
    streamer.lastUpdate = new Date();
    streamer.fragmentCount++;
    streamer.totalBytes += fragment.length;
  }

  private ensureUTF8Integrity(streamer: SonicJSONStreamer, fragment: string): string {
    if (!fragment) return fragment;

    const bytes = new TextEncoder().encode(fragment);
    const n = bytes.length;
    if (n === 0) return fragment;

    for (let i = n - 1; i >= 0 && i >= n - 4; i--) {
      const b = bytes[i];

      if ((b & 0x80) === 0) {
        break;
      } else if ((b & 0xE0) === 0xC0) {
        if (n - i < 2) {
          streamer.incompleteUTF8 = fragment.slice(i);
          return fragment.slice(0, i);
        }
        break;
      } else if ((b & 0xF0) === 0xE0) {
        if (n - i < 3) {
          streamer.incompleteUTF8 = fragment.slice(i);
          return fragment.slice(0, i);
        }
        break;
      } else if ((b & 0xF8) === 0xF0) {
        if (n - i < 4) {
          streamer.incompleteUTF8 = fragment.slice(i);
          return fragment.slice(0, i);
        }
        break;
      }
    }

    if (streamer.incompleteUTF8) {
      const combined = streamer.incompleteUTF8 + fragment;
      streamer.incompleteUTF8 = "";
      return this.ensureUTF8Integrity(streamer, combined);
    }

    return fragment;
  }

  private tryParse(streamer: SonicJSONStreamer): string {
    if (!streamer.buffer) return "empty";

    const content = streamer.buffer.trim();
    if (content === "{}" || content === "[]") {
      streamer.result = content === "{}" ? {} : null;
      streamer.hasValidJSON = true;
      return "complete";
    }

    try {
      streamer.result = JSON.parse(content);
      streamer.hasValidJSON = true;
      return "complete";
    } catch {
      return "invalid";
    }
  }

  private onAggregationComplete(toolUseId: string, fullInput: string): void {
    if (this.updateCallback) {
      this.updateCallback(toolUseId, fullInput);
    }
  }
}

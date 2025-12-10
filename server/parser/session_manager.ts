export interface SessionInfo {
  sessionId: string;
  startTime: Date;
  endTime?: Date;
}

export class SessionManager {
  private sessionId: string;
  private startTime: Date;
  private endTime?: Date;
  private isActive = false;

  constructor() {
    this.sessionId = crypto.randomUUID();
    this.startTime = new Date();
  }

  setSessionId(sessionId: string): void {
    this.sessionId = sessionId;
  }

  startSession(): Array<{ event: string; data: Record<string, unknown> }> {
    this.isActive = true;
    this.startTime = new Date();

    return [{
      event: "session_start",
      data: {
        type: "session_start",
        session_id: this.sessionId,
        timestamp: this.startTime.toISOString(),
      },
    }];
  }

  endSession(): Array<{ event: string; data: Record<string, unknown> }> {
    const now = new Date();
    this.endTime = now;
    this.isActive = false;

    return [{
      event: "session_end",
      data: {
        type: "session_end",
        session_id: this.sessionId,
        timestamp: now.toISOString(),
        duration: now.getTime() - this.startTime.getTime(),
      },
    }];
  }

  getIsActive(): boolean {
    return this.isActive;
  }

  getSessionInfo(): SessionInfo {
    return {
      sessionId: this.sessionId,
      startTime: this.startTime,
      endTime: this.endTime,
    };
  }

  reset(): void {
    this.sessionId = crypto.randomUUID();
    this.startTime = new Date();
    this.endTime = undefined;
    this.isActive = false;
  }
}

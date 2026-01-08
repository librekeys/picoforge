import { tick } from "svelte";

export type LogType = "info" | "success" | "error" | "warning";

export interface LogEntry {
  timestamp: string;
  message: string;
  type: LogType;
}

class LogSystem {
  logs = $state<LogEntry[]>([]);

  add(message: string, type: LogType = "info") {
    const now = new Date();
    const timeString = now.toLocaleTimeString("en-US", {
      hour12: false,
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });

    this.logs.push({ timestamp: timeString, message, type });
  }

  clear() {
    this.logs = [];
  }
}

export const logger = new LogSystem();

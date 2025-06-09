import {
  ACTIVE_PROCESS_STATUSES,
  BAD_PROCESS_STATUSES,
  INTERRUPTED_PROCESS_STATUSES,
  TERMINAL_PROCESS_STATUSES,
} from "@/language/core/const";
import { Transaction } from "@/language/core/transaction";
import { NodeType, ProcessableNodeData, ProcessStatus, Timestamp } from "@/proto/wire";
import { isNode } from "@/proto/wiring";
import {
  compareTimestamps,
  durationToMs,
  formatDuration,
  FormatDurationOptions,
  getNow,
  timestampToMs,
  TimeUpdateInterval,
} from "@/utils/time";

export const VERB_BY_PROCESS_STATUS: Partial<Record<ProcessStatus, string>> = {
  [ProcessStatus.UNSPECIFIED]: "???",
  [ProcessStatus.SCHEDULED]: "is scheduled",
  [ProcessStatus.RUNNING]: "is running",
  [ProcessStatus.COMPLETED]: "has completed",
  [ProcessStatus.FAILED]: "has failed",
  [ProcessStatus.ABORTED]: "was aborted",
  [ProcessStatus.CANCELLED]: "was cancelled",
  [ProcessStatus.WAITING]: "is waiting",
  [ProcessStatus.YIELDED]: "has yielded",
  [ProcessStatus.PAUSED]: "is paused",
};

export function isProcessActive(process: ProcessableNodeData): boolean {
  return ACTIVE_PROCESS_STATUSES.includes(process.status);
}

export function isProcessInterrupted(process: ProcessableNodeData): boolean {
  return INTERRUPTED_PROCESS_STATUSES.includes(process.status);
}

export function isProcessBad(process: ProcessableNodeData): boolean {
  return BAD_PROCESS_STATUSES.includes(process.status);
}

export function isProcessPaused(process: ProcessableNodeData): boolean {
  if (!isNode(process, NodeType.RUN)) return false;
  if (isProcessTerminal(process)) return false;
  return (
    process.requestedPauseAt != null &&
    (process.requestedResumeAt == null || compareTimestamps(process.requestedPauseAt, process.requestedResumeAt) > 0)
  );
}

export function isProcessTerminal(process: ProcessableNodeData): boolean {
  if (isNode(process, NodeType.SPAN)) {
    return process.terminatedAt != null;
  } else {
    return TERMINAL_PROCESS_STATUSES.includes(process.status);
  }
}

/** Touch a Process to update the activeAt timestamp */
export function touchProcess(tx: Transaction, process: ProcessableNodeData) {
  tx.update(process, { activeAt: Timestamp.now() });
}

/** Gets the startedAt timestamp of a Run */
export function getProcessStartedAtMs(process: ProcessableNodeData): number {
  return timestampToMs(process.startedAt ?? process.createdAt!);
}

/** Gets the duration of a Run */
export function getProcessDurationMs(process: ProcessableNodeData, nowMs: number): number {
  if (isProcessTerminal(process) && process.startedAt == null) {
    return 0; // never really started
  }
  const startedAtMs = getProcessStartedAtMs(process);
  let durationMs: number;
  if (process.duration != null) {
    durationMs = durationToMs(process.duration);
  } else {
    const currentMs = process.terminatedAt != null ? timestampToMs(process.terminatedAt) : nowMs;
    durationMs = currentMs - startedAtMs;
  }
  return Math.max(0, durationMs);
}

/** Gets the duration as a formatted string */
export function getProcessDurationString(process: ProcessableNodeData, options?: FormatDurationOptions): string | null {
  const now = getNow(TimeUpdateInterval.MILLISECOND).value;
  const nowMs = timestampToMs(now);
  const durationMs = getProcessDurationMs(process, nowMs);
  if (durationMs == 0) return null;
  return formatDuration(durationMs, options);
}

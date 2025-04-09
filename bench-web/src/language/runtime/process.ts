import {
  ACTIVE_PROCESS_STATUSES,
  BAD_PROCESS_STATUSES,
  INTERRUPTED_PROCESS_STATUSES,
  TERMINAL_PROCESS_STATUSES,
} from "@/language/core/const";
import {
  FieldType,
  FlowData,
  InterruptionData,
  LinkData,
  NodeType,
  ObjectType,
  PropertyReferenceData,
  RunProperty,
  SpanData,
  ProcessStatus,
  RunType,
  type ActionData,
  type RunData,
  RUNNABLE_NODE_TYPES,
  RunnableNodeData,
  ProcessableNodeData,
} from "@/proto/wire";
import { describeNode, isNode, propertyReference } from "@/proto/wiring";
import { assertNever } from "@/utils/functools";
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

export const RUN_PROPERTY_BY_FIELD_TYPE: Partial<Record<FieldType, PropertyReferenceData>> = {
  [FieldType.OUTPUT]: propertyReference(ObjectType.RUN, RunProperty.outputsPacked),
  [FieldType.INPUT]: propertyReference(ObjectType.RUN, RunProperty.inputsPacked),
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

export const RUN_TYPE_BY_NODE_TYPE: Partial<Record<NodeType, RunType>> = {
  [NodeType.FLOW]: RunType.FLOW,
  [NodeType.ACTION]: RunType.ACTION,
  [NodeType.LINK]: RunType.LINK,
};
export const NODE_TYPE_BY_RUN_TYPE: Partial<Record<RunType, NodeType>> = {
  [RunType.FLOW]: NodeType.FLOW,
  [RunType.ACTION]: NodeType.ACTION,
  [RunType.LINK]: NodeType.LINK,
};

/** Determine the type of run for some runnable object */
export function getRunType(runnable: RunnableNodeData): RunType {
  const runType = RUN_TYPE_BY_NODE_TYPE[runnable.metatype as unknown as keyof typeof RUN_TYPE_BY_NODE_TYPE];
  if (runType == null) {
    throw new Error(`unexpected runnable type: ${describeNode(runnable)}`);
  }
  return runType;
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

/** Gets the duration of an Interrupt */
export function getInterruptDurationMs(interrupt: InterruptionData, nowMs: number): number {
  const startedAtMs = timestampToMs(interrupt.createdAt!);
  const closedAtMs = interrupt.closedAt != null ? timestampToMs(interrupt.closedAt) : nowMs;
  return closedAtMs - startedAtMs;
}

/** Gets the duration of an Interrupt as a formatted string */
export function getInterruptDurationString(
  interrupt: InterruptionData,
  options?: FormatDurationOptions,
): string | null {
  const now = getNow(TimeUpdateInterval.MILLISECOND).value;
  const nowMs = timestampToMs(now);
  const durationMs = getInterruptDurationMs(interrupt, nowMs);
  if (durationMs == 0) return null;
  return formatDuration(durationMs, options);
}

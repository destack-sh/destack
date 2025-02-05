import {
  ACTIVE_RUN_STATUSES,
  BAD_RUN_STATUSES,
  INTERRUPTED_RUN_STATUSES,
  TERMINAL_RUN_STATUSES,
} from "@/language/const";
import {
  BlockType,
  CodeData,
  FieldType,
  FlowData,
  InterruptionData,
  NodeType,
  ObjectType,
  PipeData,
  PropertyReferenceData,
  RunProperty,
  RunSpanData,
  RunStatus,
  RunType,
  TextData,
  type ActionData,
  type BlockData,
  type RunData,
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

export type RunnableNode = FlowData | ActionData | PipeData;
export type RunnableNodeType = NodeType.FLOW | NodeType.ACTION | NodeType.PIPE;

export const VERB_BY_RUN_STATUS: Partial<Record<RunStatus, string>> = {
  [RunStatus.UNSPECIFIED]: "???",
  [RunStatus.SCHEDULED]: "is scheduled",
  [RunStatus.RUNNING]: "is running",
  [RunStatus.COMPLETED]: "has completed",
  [RunStatus.FAILED]: "has failed",
  [RunStatus.ABORTED]: "was aborted",
  [RunStatus.CANCELLED]: "was cancelled",
  [RunStatus.WAITING]: "is waiting",
  [RunStatus.YIELDED]: "has yielded",
  [RunStatus.PAUSED]: "is paused",
};

export const RUN_PROPERTY_BY_FIELD_TYPE: Partial<Record<FieldType, PropertyReferenceData>> = {
  [FieldType.VARIABLE]: propertyReference(ObjectType.RUN, RunProperty.variablesPacked),
  [FieldType.OUTPUT]: propertyReference(ObjectType.RUN, RunProperty.outputsPacked),
  [FieldType.INPUT]: propertyReference(ObjectType.RUN, RunProperty.inputsPacked),
};

/** Whether the given node is runnable */
export function isRunnable(node: any | null | undefined): node is RunnableNode {
  if (isNode(node, NodeType.ACTION)) {
    return true;
  } else if (isNode(node, NodeType.BLOCK)) {
    return node.type == BlockType.FLOW;
  } else {
    return false;
  }
}

export function isRunActive(run: RunData | RunSpanData): boolean {
  return ACTIVE_RUN_STATUSES.includes(run.status);
}

export function isRunInterrupted(run: RunData | RunSpanData): boolean {
  return INTERRUPTED_RUN_STATUSES.includes(run.status);
}

export function isRunBad(run: RunData | RunSpanData): boolean {
  return BAD_RUN_STATUSES.includes(run.status);
}

export function isRunPaused(run: RunData | RunSpanData): boolean {
  if (!isNode(run, NodeType.RUN)) return false;
  if (isRunTerminal(run)) return false;
  return run.pausedAt != null && (run.resumedAt == null || compareTimestamps(run.pausedAt, run.resumedAt) > 0);
}

export function isRunTerminal(run: RunData | RunSpanData): boolean {
  if (isNode(run, NodeType.RUN)) {
    return TERMINAL_RUN_STATUSES.includes(run.status);
  } else if (isNode(run, NodeType.RUN_SPAN)) {
    return run.terminatedAt != null;
  } else {
    assertNever(run);
  }
}

/** Determine the type of run for some runnable object */
export function getRunType(runnable: RunnableNode): RunType {
  if (isNode(runnable, NodeType.BLOCK)) {
    if (runnable.type == BlockType.FLOW) {
      return RunType.FLOW;
    }
  } else if (isNode(runnable, NodeType.ACTION)) {
    return RunType.ACTION;
  }

  throw new Error(`unexpected runnable type: ${describeNode(runnable)}`);
}

/** Gets the startedAt timestamp of a Run */
export function getRunStartedAtMs(run: RunData | RunSpanData): number {
  if (isNode(run, NodeType.RUN)) {
    return timestampToMs(run.startedAt ?? run.createdAt!);
  } else if (isNode(run, NodeType.RUN_SPAN)) {
    return timestampToMs(run.startedAt!);
  } else {
    assertNever(run);
  }
}

/** Gets the duration of a Run */
export function getRunDurationMs(run: RunData | RunSpanData, nowMs: number): number {
  if (isRunTerminal(run) && run.startedAt == null) {
    return 0; // never really started
  }
  const startedAtMs = getRunStartedAtMs(run);
  let durationMs: number;
  if (run.duration != null) {
    durationMs = durationToMs(run.duration);
  } else {
    const currentMs = run.terminatedAt != null ? timestampToMs(run.terminatedAt) : nowMs;
    durationMs = currentMs - startedAtMs;
  }
  return Math.max(0, durationMs);
}

/** Gets the Run duration as a formatted string */
export function getRunDurationString(run: RunData, options?: FormatDurationOptions): string | null {
  const now = getNow(TimeUpdateInterval.MILLISECOND).value;
  const nowMs = timestampToMs(now);
  const durationMs = getRunDurationMs(run, nowMs);
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

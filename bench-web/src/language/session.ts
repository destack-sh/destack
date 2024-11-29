import { ACTIVE_RUN_STATUSES, INTERRUPTED_RUN_STATUSES, TERMINAL_RUN_STATUSES } from "@/language/const";
import type { ReadNodeGraph } from "@/language/graph";
import { makeNode } from "@/language/node";
import {
  BlockType,
  CodeData,
  InterruptData,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PipeData,
  RunType,
  RunOptionsData,
  RunStatus,
  StructType,
  TextData,
  type AnyNodeData,
  type BlockData,
  type RunData,
  type StepData,
  RuntimeMode,
} from "@/proto/wire";
import { describeNode, isNode, isStruct, makeDefaultObject, toNodeRef } from "@/proto/wiring";
import { space } from "@/system/space";
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

export type RunnableNode = BlockData | StepData | PipeData;
export type RunnableNodeType = NodeType.BLOCK | NodeType.STEP | NodeType.PIPE;
export type RunnableObject = RunnableNode | TextData | CodeData;

/** Whether the given node is runnable */
export function isRunnable(node: any | null | undefined): node is RunnableNode {
  if (isNode(node, NodeType.STEP)) {
    return true;
  } else if (isNode(node, NodeType.BLOCK)) {
    return node.type == BlockType.ACTION || node.type == BlockType.FLOW;
  } else {
    return false;
  }
}

export function isRunActive(run: RunData): boolean {
  return ACTIVE_RUN_STATUSES.includes(run.status);
}

export function isRunInterrupted(run: RunData): boolean {
  return INTERRUPTED_RUN_STATUSES.includes(run.status);
}

export function isRunPaused(run: RunData): boolean {
  if (isRunTerminal(run)) return false;
  return run.pausedAt != null && (run.resumedAt == null || compareTimestamps(run.pausedAt, run.resumedAt) > 0);
}

export function isRunTerminal(run: RunData): boolean {
  return TERMINAL_RUN_STATUSES.includes(run.status);
}

export function getRunBasePtr(run: RunData): NodeReferenceData | null {
  return run.pipePtr ?? run.stepPtr ?? run.blockPtr ?? null;
}

export function getInterruptBasePtr(interrupt: InterruptData): NodeReferenceData | null {
  return interrupt.pipePtr ?? interrupt.stepPtr ?? interrupt.blockPtr ?? null;
}

/** Determine the type of run for some runnable object */
export function getRunType(runnable: RunnableObject): RunType {
  if (isNode(runnable, NodeType.BLOCK)) {
    if (runnable.type == BlockType.ACTION) {
      return RunType.ACTION;
    } else if (runnable.type == BlockType.FLOW) {
      return RunType.FLOW;
    }
  } else if (isNode(runnable, NodeType.STEP)) {
    return RunType.STEP;
  }

  throw new Error(`unexpected runnable type: ${describeNode(runnable)}`);
}

/** Gets the duration of a Run */
export function getRunDurationMs(run: RunData, nowMs: number): number {
  if (TERMINAL_RUN_STATUSES.includes(run.status) && run.startedAt == null) {
    return 0; // never really started
  }
  const startedAtMs = timestampToMs(run.startedAt ?? run.createdAt!);
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
export function getInterruptDurationMs(interrupt: InterruptData, nowMs: number): number {
  const startedAtMs = timestampToMs(interrupt.createdAt!);
  const closedAtMs = interrupt.closedAt != null ? timestampToMs(interrupt.closedAt) : nowMs;
  return closedAtMs - startedAtMs;
}

/** Gets the duration of an Interrupt as a formatted string */
export function getInterruptDurationString(interrupt: InterruptData, options?: FormatDurationOptions): string | null {
  const now = getNow(TimeUpdateInterval.MILLISECOND).value;
  const nowMs = timestampToMs(now);
  const durationMs = getInterruptDurationMs(interrupt, nowMs);
  if (durationMs == 0) return null;
  return formatDuration(durationMs, options);
}

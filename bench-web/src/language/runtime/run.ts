import { InterruptionData, NodeType, RunnableNodeData, RunType } from "@/proto/wire";
import { describeNode } from "@/proto/wiring";
import { formatDuration, FormatDurationOptions, getNow, timestampToMs, TimeUpdateInterval } from "@/utils/time";

export const RUN_TYPE_BY_NODE_TYPE: Partial<Record<NodeType, RunType>> = {
  [NodeType.FLOW]: RunType.FLOW,
  [NodeType.ACTION]: RunType.ACTION,
  [NodeType.TRANSITION]: RunType.TRANSITION,
};
export const NODE_TYPE_BY_RUN_TYPE: Partial<Record<RunType, NodeType>> = {
  [RunType.FLOW]: NodeType.FLOW,
  [RunType.ACTION]: NodeType.ACTION,
  [RunType.TRANSITION]: NodeType.TRANSITION,
};

/** Determine the type of run for some runnable object */
export function getRunType(runnable: RunnableNodeData): RunType {
  const runType = RUN_TYPE_BY_NODE_TYPE[runnable.metatype as unknown as keyof typeof RUN_TYPE_BY_NODE_TYPE];
  if (runType == null) {
    throw new Error(`unexpected runnable type: ${describeNode(runnable)}`);
  }
  return runType;
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

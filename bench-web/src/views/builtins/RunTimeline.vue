<script lang="ts" setup>
import { getRunBase as getRunBasePtr, getRunDurationMs, isRunTerminal } from "@/language/session";
import {
  AnyNodeData,
  IconData,
  NodeType,
  RunAttemptData,
  RunData,
  RunSpanData,
  RunStatus,
  ViewData,
} from "@/proto/wire";
import { describeNode, isNode, SomeNodeReferenceData, TypedNodeReferenceData } from "@/proto/wiring";
import { RunTree } from "@/system/runtime";
import { canvas, pkgGraph } from "@/system/space";
import { getNodeIcon, ICON_BY_NODE_TYPE, ICON_BY_RUN_STATUS, IconInline } from "@/ui/icon";
import { COLOR_BY_RUN_STATUS, getColorHex } from "@/ui/style";
import { formatDuration, getNow, timestampToMs, TimeUpdateInterval } from "@/utils/time";
import { useElementSize } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, ref, Ref, shallowRef, toRef, watchEffect } from "vue";

const TREE_WIDTH = 280;
const DEPTH_OFFSET = 12;
const ROW_HEIGHT = 28;
const BAR_PADDING = 4;
const MIN_SPAN_WIDTH = 2;

const props = defineProps<{ nodePtr: SomeNodeReferenceData } & Pick<ViewData, "size">>();
const nodePtr = toRef(props, "nodePtr") as Ref<TypedNodeReferenceData<NodeType.RUN> | null>;

const containerRef = ref<HTMLElement | null>(null);
const { width: containerWidth, height: containerHeight } = useElementSize(containerRef);
const spanContainerWidth = computed(() => containerWidth.value - TREE_WIDTH);

//
// Run
//

const runTree = new RunTree(pkgGraph, nodePtr);

//
// Spans / Events
//

type Timeline = {
  root: RunData | null;
  spans: TimelineSpan[];
  events: TimelineEvent[];
  hasActive: boolean; // whether there are any still active spans in the timeline
};
const EMPTY_TIMELINE: Timeline = { root: null, spans: [], events: [], hasActive: false };

type TimelineSpan = {
  id: string;
  parent: TimelineSpan | null | undefined;
  depth: number;
  icon: IconData | null | undefined;
  color: string;
  title: string;
  startedAtMs: number;
  durationMs: number;
  baseNode: AnyNodeData | null | undefined;
  content: RunData | RunAttemptData | RunSpanData;
  offsetRelative: number;
  widthRelative: number;
  isActive: boolean;
};

type TimelineEvent = {
  id: string;
  icon: IconData | null | undefined;
  title: string;
  atMs: number;
};

function getStartedAtMs(run: RunData): number {
  return timestampToMs(run.startedAt ?? run.createdAt!);
}

function makeTimeline(now: DateTime, root: RunData): Timeline {
  const spans: TimelineSpan[] = [];
  const events: TimelineEvent[] = [];

  const nowMs = timestampToMs(now);
  const rootStartedAtMs = getStartedAtMs(root);
  const rootDurationMs = getRunDurationMs(root, nowMs);

  function walkRun(run: RunData, parent: TimelineSpan | null, depth: number) {
    // timing
    const startedAtMs = getStartedAtMs(run);
    const durationMs = getRunDurationMs(run, nowMs);

    // context
    const basePtr = getRunBasePtr(run);
    const baseNode = basePtr != null ? pkgGraph.get(basePtr) : null;
    const color = getColorHex(COLOR_BY_RUN_STATUS[run.status])!;

    // span
    const offsetRelative = Math.max(0, Math.min(1, (startedAtMs - rootStartedAtMs) / rootDurationMs));
    const widthRelative = Math.max(0, Math.min(1 - offsetRelative, durationMs / rootDurationMs));
    const span: TimelineSpan = {
      id: run.id,
      parent: parent,
      depth: depth,
      icon: baseNode != null ? getNodeIcon(baseNode) : null,
      color,
      title: (baseNode as any)?.name ?? "Run",
      startedAtMs: startedAtMs,
      durationMs: durationMs,
      baseNode: baseNode,
      content: run,
      offsetRelative,
      widthRelative,
      isActive: !isRunTerminal(run),
    };
    spans.push(span);

    // descend
    for (const child of runTree.runGraph.getChildren(run)) {
      if (!isNode(child, NodeType.RUN)) continue;
      walkRun(child, span, depth + 1);
    }
  }

  walkRun(root, null, 0);

  const hasActive = spans.some((span) => span.isActive);
  return { root, spans, events, hasActive };
}

const now = getNow(TimeUpdateInterval.MILLISECOND);
const timeline: Ref<Timeline> = shallowRef(EMPTY_TIMELINE);
watchEffect(() => {
  if (timeline.value?.root?.id != runTree.run?.id || !runTree.runs.every(isRunTerminal) || timeline.value?.hasActive) {
    if (runTree.run != null) {
      // NOTE :Robustness: technically, we shouldn't need to re-fetch the root node, the ref should just work :NodeRefStability
      //  (but right now, it doesn't always work and that looks really bad.. so we re-fetch the root node)
      const root = runTree.runGraph.get(runTree.run);
      if (!isNode(root, NodeType.RUN)) throw new Error(`expected run node for ${describeNode(runTree.run)}`);
      timeline.value = makeTimeline(now.value, root);
    } else {
      timeline.value = EMPTY_TIMELINE;
    }
  }
});
</script>
<template>
  <!-- Spans -->
  <!-- NOTE :Incomplete: RunTimeline 'axis' markers above spans (regularly spaced) -->
  <div ref="containerRef" class="flex w-full flex-1 flex-col">
    <!-- Span -->
    <div
      v-for="span in timeline.spans"
      :key="span.id"
      class="group/span relative flex w-full flex-row items-center rounded hover:bg-gray-100"
      :style="{
        height: ROW_HEIGHT + 'px',
      }"
      :data-node-type="span.baseNode?.metatype"
      :data-node-id="span.baseNode?.id"
      :data-node-ck="(span.baseNode as any)?.ck"
    >
      <!-- Tree  -->
      <div
        class="flex flex-shrink-0 flex-row items-center pr-2"
        :style="{
          paddingLeft: span.depth * DEPTH_OFFSET + 'px',
          width: TREE_WIDTH + 'px',
        }"
      >
        <!-- Node -->
        <button
          class="group/node truncate hover:cursor-pointer"
          @click.stop="isNode(span.baseNode) && canvas.goToNode(span.baseNode)"
        >
          <IconInline
            v-bind="span.icon ?? ICON_BY_NODE_TYPE[NodeType.RUN]"
            class="mr-1.5 w-5 text-center text-gray-700 transition-colors duration-75"
          />
          <span class="truncate underline-offset-3 group-hover/node:underline">{{ span.title }}</span>
        </button>
        <!-- Meta -->
        <div class="ml-auto flex-shrink-0 pl-1.5">
          <!-- Duration -->
          <span class="ml-auto mr-1.5 text-gray-400">{{ formatDuration(span.durationMs, { minUnit: "s" }) }}</span>
          <!-- Status -->
          <IconInline
            v-if="isNode(span.content)"
            class="ml-auto w-5 text-center"
            :class="[span.content.status == RunStatus.RUNNING ? 'animate-spin' : '']"
            :style="{ color: span.color }"
            v-bind="ICON_BY_RUN_STATUS[span.content.status]"
          />
        </div>
      </div>
      <!-- Timeline -->
      <div
        class="absolute transform rounded transition-all duration-100"
        :style="{
          height: ROW_HEIGHT - 2 * BAR_PADDING + 'px',
          top: BAR_PADDING + 'px',
          width: Math.max(MIN_SPAN_WIDTH, span.widthRelative * spanContainerWidth) + 'px',
          left: TREE_WIDTH + span.offsetRelative * spanContainerWidth + 'px',
          backgroundColor: span.color,
        }"
        @click.stop="isNode(span.baseNode) && canvas.goToNode(span.baseNode)"
      ></div>
    </div>
  </div>
</template>

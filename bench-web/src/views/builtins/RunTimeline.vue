<script lang="ts" setup>
import { getBaseFromNode, toCamelName } from "@/language/const";
import { ReadNodeGraph } from "@/language/graph";
import { getRunDurationMs, getRunStartedAtMs, isRunTerminal } from "@/language/session";
import {
  AnyNodeData,
  ColorShade,
  ColorType,
  IconData,
  LogLevel,
  NodeReferenceData,
  NodeType,
  Orientation,
  RunAttemptData,
  RunData,
  RunSpanData,
  RunSpanType,
  StructType,
  ViewData,
} from "@/proto/wire";
import { describeNode, isNode, isStruct, TypedNodeReferenceData } from "@/proto/wiring";
import { RunTree } from "@/system/runtime";
import { canvas } from "@/system/space";
import { getNodeIcon, ICON_BY_NODE_TYPE, ICON_BY_RUN_SPAN_TYPE, IconInline } from "@/ui/icon";
import { COLOR_BY_RUN_STATUS, getColorHex } from "@/ui/style";
import { assertNever } from "@/utils/functools";
import { formatDuration, getNow, timestampToMs, TimeUpdateInterval } from "@/utils/time";
import RunStatus from "@/views/builtins/RunStatus.vue";
import { useElementSize } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, onMounted, ref, Ref, shallowRef, toRef, watchEffect } from "vue";

const DEPTH_OFFSET = 12;
const ROW_HEIGHT = 28;
const BAR_PADDING = 4;
const MIN_SPAN_WIDTH = 4;
const BASE_TYPES = [NodeType.BLOCK, NodeType.ACTION]; // NOTE :Incomplete: RunTimeline should be configurable

const props = defineProps<{ graph: ReadNodeGraph; nodePtr: NodeReferenceData } & Pick<ViewData, "size">>();
const nodePtr = toRef(props, "nodePtr") as Ref<TypedNodeReferenceData<NodeType.RUN> | null>;

const containerRef = ref<HTMLElement | null>(null);
const { width: containerWidth, height: containerHeight } = useElementSize(containerRef);
const spanContainerWidth = computed(() => containerWidth.value);

// wait for initial render to complete so the transition-all doesn't look glitchy on mount
const isInitialRender = ref(true);
onMounted(() => {
  setTimeout(() => {
    isInitialRender.value = false;
  }, 100);
});

//
// Run
//

const runTree = new RunTree(props.graph, nodePtr);
const minLevel: Ref<LogLevel> = ref(LogLevel.INFO);

type Timeline = {
  root: RunData | null;
  spans: TimelineNode[];
  events: TimelineEvent[];
  hasActive: boolean; // whether there are any still active spans in the timeline
};
const EMPTY_TIMELINE: Timeline = { root: null, spans: [], events: [], hasActive: false };

type TimelineNode = {
  id: string;
  parent: TimelineNode | null | undefined;
  depth: number;
  icon: IconData | null | undefined;
  color: string;
  name: string;
  startedAtMs: number;
  durationMs: number;
  baseNode: AnyNodeData | null | undefined;
  content: RunData | RunAttemptData | RunSpanData;
  offsetRelative: number;
  durationRelative: number;
  isActive: boolean;
};

type TimelineEvent = {
  id: string;
  icon: IconData | null | undefined;
  title: string;
  atMs: number;
};

function makeTimeline(now: DateTime, root: RunData): Timeline {
  const spans: TimelineNode[] = [];
  const events: TimelineEvent[] = [];

  const nowMs = timestampToMs(now);
  const rootStartedAtMs = getRunStartedAtMs(root);
  const rootDurationMs = getRunDurationMs(root, nowMs);

  function walkRun(run: RunData | RunSpanData, parent: TimelineNode | null, depth: number) {
    // timing
    const startedAtMs = getRunStartedAtMs(run);
    const durationMs = getRunDurationMs(run, nowMs);

    // context
    const basePtr = isNode(run, NodeType.RUN) ? getBaseFromNode(run) : null;
    const baseNode = basePtr != null ? props.graph.get(basePtr) : null;
    const color = !isStruct(run, StructType.RUN_SPAN)
      ? getColorHex(COLOR_BY_RUN_STATUS[run.status], ColorShade.S500)!
      : getColorHex(ColorType.SUCCESS, ColorShade.S500)!;
    let icon: IconData | null | undefined = null;
    let name: string = "???";
    if (isNode(run, NodeType.RUN)) {
      icon = baseNode != null ? getNodeIcon(baseNode) : ICON_BY_NODE_TYPE[NodeType.RUN];
      name = (baseNode as any)?.name ?? "Run";
    } else if (isStruct(run, StructType.RUN_SPAN)) {
      icon = ICON_BY_RUN_SPAN_TYPE[run.type];
      name = toCamelName(RunSpanType, run.type);
    } else {
      assertNever(run);
    }

    // span
    let offsetRelative: number;
    let durationRelative: number;
    if (rootDurationMs != 0) {
      offsetRelative = Math.max(0, Math.min(1, (startedAtMs - rootStartedAtMs) / rootDurationMs));
      if (durationMs != 0) {
        durationRelative = Math.max(0, Math.min(1 - offsetRelative, durationMs / rootDurationMs));
      } else {
        durationRelative = 0;
      }
    } else {
      offsetRelative = 0;
      durationRelative = 1;
    }
    const span: TimelineNode = {
      id: isNode(run, NodeType.RUN) ? run.id : spans.length.toString(),
      parent: parent,
      depth: depth,
      icon,
      color,
      name,
      startedAtMs: startedAtMs,
      durationMs: durationMs,
      baseNode: baseNode,
      content: run,
      offsetRelative,
      durationRelative: durationRelative,
      isActive: !isRunTerminal(run),
    };
    spans.push(span);

    // descend
    if (isNode(run, NodeType.RUN)) {
      for (const child of run.spans) {
        if (child.level < minLevel.value) continue;
        walkRun(child, span, depth + 1);
      }
      for (const child of runTree.runGraph.getChildren(run)) {
        if (!isNode(child, NodeType.RUN)) continue;
        const basePtr = getBaseFromNode(child);
        if (basePtr != null && !BASE_TYPES.includes(basePtr.nodeType)) continue;
        walkRun(child, span, depth + 1);
      }
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
  <!-- TODO :Incomplete: RunTimeline linear view -->
  <div ref="containerRef" class="flex w-full flex-1 flex-col gap-y-0.5">
    <!-- Span -->
    <div
      v-for="span in timeline.spans"
      :key="span.id"
      class="group/span relative w-full rounded transition-colors duration-75"
      :class="[canvas.isHighlighted(span.baseNode) ? 'bg-gray-100' : 'bg-white hover:bg-gray-100']"
      :style="{
        height: ROW_HEIGHT + 'px',
      }"
      :data-node-type="span.baseNode?.metatype"
      :data-node-id="span.baseNode?.id"
      :data-node-ck="(span.baseNode as any)?.ck"
    >
      <!-- Tree  -->
      <div
        class="relative flex w-full flex-row items-center py-1 pr-2"
        :style="{
          paddingLeft: span.depth * DEPTH_OFFSET + 'px',
        }"
      >
        <!-- Timeline -->
        <div
          class="absolute bottom-0 h-[3px] w-full transform rounded-sm bg-red-500"
          :class="{ 'transition-all duration-100': !isInitialRender }"
          :style="{
            width: Math.max(MIN_SPAN_WIDTH, span.durationRelative * spanContainerWidth) + 'px',
            left:
              (span.durationRelative * spanContainerWidth < MIN_SPAN_WIDTH
                ? Math.max(
                    0,
                    span.offsetRelative * spanContainerWidth -
                      (MIN_SPAN_WIDTH - span.durationRelative * spanContainerWidth),
                  )
                : span.offsetRelative * spanContainerWidth) + 'px',
            backgroundColor: span.color,
          }"
        />
        <!-- Node -->
        <button
          class="group/node truncate hover:cursor-pointer"
          @click.stop="isNode(span.baseNode) && canvas.goToNode(span.baseNode)"
        >
          <IconInline
            v-bind="span.icon ?? ICON_BY_NODE_TYPE[NodeType.RUN]"
            class="mr-1.5 w-5 text-center text-gray-700 transition-colors duration-75"
          />
          <span class="truncate underline-offset-3 group-hover/node:underline">{{ span.name }}</span>
        </button>
        <!-- Meta -->
        <div class="ml-auto flex-shrink-0 pl-1.5">
          <RunStatus
            v-if="isNode(span.content, NodeType.RUN)"
            :run="span.content"
            :orientation="Orientation.HORIZONTAL_REVERSED"
          />
          <div v-else class="flex flex-row items-center gap-x-1.5">
            <!-- Poor man's RunStatus for non-Run items -->
            <!-- Duration -->
            <span class="text-gray-400">{{ formatDuration(span.durationMs, { minUnit: "s" }) }}</span>
            <span
              class="h-[8px] w-[8px] rounded-full transition-colors duration-150"
              :style="{ backgroundColor: span.color }"
            />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

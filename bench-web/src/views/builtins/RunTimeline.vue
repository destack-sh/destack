<script lang="ts" setup>
import { getBaseFromNode, toCamelName } from "@/language/const";
import { ReadNodeGraph } from "@/language/graph";
import { getRunDurationMs, getRunStartedAtMs, isRunActive, isRunTerminal, RunnableNode } from "@/language/session";
import {
  AnyNodeData,
  ColorShade,
  ColorType,
  IconData,
  LogLevel,
  RunStatus,
  NodeReferenceData,
  NodeType,
  Orientation,
  RunData,
  RunSpanData,
  RunSpanType,
  ViewData,
} from "@/proto/wire";
import { describeNode, isNode, TypedNodeReferenceData } from "@/proto/wiring";
import { getInputType, getOutputType, RunTree } from "@/system/runtime";
import { canvas } from "@/system/space";
import { getNodeIcon, ICON_BY_NODE_TYPE, ICON_BY_RUN_SPAN_TYPE, ICON_BY_RUN_STATUS, IconInline } from "@/ui/icon";
import { COLOR_BY_RUN_STATUS, getColorHex } from "@/ui/style";
import { assertNever } from "@/utils/functools";
import { formatDuration, getNow, timestampToMs, TimeUpdateInterval } from "@/utils/time";
import RunStatusView from "@/views/builtins/RunStatus.vue";
import { useElementSize } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, onMounted, ref, Ref, shallowRef, toRef, watchEffect } from "vue";
import SomeObject from "@/views/objects/Object.vue";

const DEPTH_OFFSET = 12;
const ROW_HEIGHT = 28;
const BAR_PADDING = 4;
const MIN_SPAN_WIDTH = 4;
const BASE_TYPES = [NodeType.BLOCK, NodeType.ACTION]; // NOTE :Incomplete: RunTimeline should be configurable

const props = defineProps<
  { graph: ReadNodeGraph; nodePtr: NodeReferenceData; layout: "linear" | "tree" } & Pick<ViewData, "size">
>();
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
  spans: TimelineSpan[];
  events: TimelineEvent[];
  descendants: (TimelineSpan | TimelineEvent)[];
  hasActive: boolean; // whether there are any still active spans in the timeline
};
const EMPTY_TIMELINE: Timeline = { root: null, spans: [], events: [], descendants: [], hasActive: false };

type TimelineSpan = {
  metatype: "span";
  id: string;
  parent: TimelineSpan | null | undefined;
  depth: number;
  icon: IconData;
  color: string;
  name: string;
  startedAtMs: number;
  durationMs: number;
  baseNode: AnyNodeData | null | undefined;
  content: RunData | RunSpanData;
  offsetRelative: number;
  durationRelative: number;
  isActive: boolean;
};

type TimelineEvent = {
  metatype: "event";
  id: string;
  kind: "start";
  icon: IconData | null | undefined;
  startedAtMs: number;
  title: string;
  atMs: number;
};

function makeTimeline(now: DateTime, root: RunData): Timeline {
  const spans: TimelineSpan[] = [];
  const events: TimelineEvent[] = [];

  const nowMs = timestampToMs(now);
  const rootStartedAtMs = getRunStartedAtMs(root);
  const rootDurationMs = getRunDurationMs(root, nowMs);

  function walkRun(run: RunData | RunSpanData, parent: TimelineSpan | null, depth: number) {
    // timing
    const startedAtMs = getRunStartedAtMs(run);
    const durationMs = getRunDurationMs(run, nowMs);

    // context
    const basePtr = isNode(run, NodeType.RUN) ? getBaseFromNode(run) : null;
    const baseNode = basePtr != null ? props.graph.get(basePtr) : null;
    const color = !isNode(run, NodeType.RUN_SPAN)
      ? getColorHex(COLOR_BY_RUN_STATUS[run.status], ColorShade.S500)!
      : getColorHex(ColorType.SUCCESS, ColorShade.S500)!;
    let icon: IconData;
    let name: string = "???";
    if (isNode(run, NodeType.RUN)) {
      icon = (baseNode != null ? getNodeIcon(baseNode) : null) ?? ICON_BY_NODE_TYPE[NodeType.RUN]!;
      name = (baseNode as any)?.name ?? "Run";
    } else if (isNode(run, NodeType.RUN_SPAN)) {
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
    const span: TimelineSpan = {
      metatype: "span",
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
      for (const child of runTree.runGraph.getChildren(run)) {
        if (isNode(child, NodeType.RUN)) {
          const basePtr = getBaseFromNode(child);
          if (basePtr != null && !BASE_TYPES.includes(basePtr.nodeType)) continue;
          walkRun(child, span, depth + 1);
        } else if (isNode(child, NodeType.RUN_SPAN)) {
          walkRun(child, span, depth + 1);
        }
      }
    }
  }

  walkRun(root, null, 0);

  const descendants = [...spans.slice(1) /* skip root */, ...events];
  descendants.sort((a, b) => a.startedAtMs - b.startedAtMs);
  const hasActive = spans.some((span) => span.isActive);
  return { root, spans, events, descendants, hasActive };
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
  <div v-if="layout == 'linear'" ref="containerRef" class="flex w-full flex-1 flex-col gap-y-1">
    <!-- Linear -->
    <div v-for="thing in timeline.descendants" :key="thing.id" class="w-full">
      <!-- Run -->
      <div v-if="thing.metatype == 'span' && isNode(thing.content, NodeType.RUN)" class="w-full py-0.5">
        <!-- Header -->
        <div class="flex w-full flex-row items-center">
          <!-- Node -->
          <button
            class="group/node truncate hover:cursor-pointer"
            @click.stop="isNode(thing.baseNode) && canvas.goToNode(thing.baseNode)"
          >
            <!-- Icon (from Run if active) -->
            <IconInline
              v-if="isRunActive(thing.content)"
              v-bind="ICON_BY_RUN_STATUS[thing.content.status]"
              class="mr-1.5 w-5 text-center text-gray-700"
              :class="[thing.content.status == RunStatus.RUNNING ? 'animate-spin' : '']"
            />
            <IconInline v-else v-bind="thing.icon" class="mr-1.5 w-5 text-center text-gray-700" />
            <span class="truncate font-medium underline-offset-3 group-hover/node:underline">{{ thing.name }}</span>
          </button>
          <!-- Status -->
          <RunStatusView class="ml-auto" :run="thing.content" :orientation="Orientation.HORIZONTAL_REVERSED" />
        </div>
        <!-- Content -->
        <div>
          <SomeObject
            id="fields-variables"
            class="w-full"
            :value-type="getInputType(thing.baseNode as RunnableNode)"
            is-inline
            is-minimal
            :model-value="thing.content.inputsPacked"
          />
          <SomeObject
            id="fields-variables"
            class="w-full"
            :value-type="getOutputType(thing.baseNode as RunnableNode)"
            is-inline
            is-minimal
            :model-value="thing.content.outputsPacked"
          />
        </div>
      </div>
      <!-- Event -->
      <div v-else-if="thing.metatype == 'event'">
        <div>
          <span>{{ thing.title }}</span>
        </div>
      </div>
    </div>
  </div>
  <div v-else ref="containerRef" class="flex w-full flex-1 flex-col gap-y-0.5">
    <!-- Tree -->
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

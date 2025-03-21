<script lang="ts" setup>
import { getBaseFromNode, toCamelName } from "@/language/core/const";
import { ReadNodeGraph } from "@/language/core/graph";
import {
  getRunDurationMs,
  getRunDurationString,
  getRunStartedAtMs,
  isRunActive,
  isRunBad,
  isRunInterrupted,
  isRunTerminal,
  RunnableNode,
} from "@/language/runtime/run";
import {
  ActionData,
  ActionType,
  AnyNodeData,
  ColorShade,
  ColorType,
  IconData,
  InterruptionData,
  InterruptionStatus,
  NodeReferenceData,
  NodeType,
  NodeTypeOptionInfo,
  Orientation,
  RunData,
  SpanData,
  SpanType,
  SpanTypeOptionInfo,
  RunStatus,
  RunStatusOptionInfo,
  Severity,
  ViewData,
} from "@/proto/wire";
import { describeNode, isNode, TypedNodeReferenceData } from "@/proto/wiring";
import { getInputType, getOutputType, getRunCommands, runtime, RunTree } from "@/runtime/runtime";
import { canvas } from "@/system/space";
import { getNodeIcon, IconInline, makeIcon } from "@/ui/icon";
import { getColorHex, getRunColorHex } from "@/ui/style";
import { assertNever } from "@/utils/functools";
import { IS_DEVELOPER_MODE } from "@/utils/globals";
import { formatDuration, getNow, timestampToMs, TimeUpdateInterval } from "@/utils/time";
import Error from "@/views/builtins/Error.vue";
import Code from "@/views/content/Code.vue";
import SomeObject from "@/views/objects/Object.vue";
import { useElementSize } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, onMounted, ref, Ref, shallowRef, toRef, watchEffect } from "vue";

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
const codeExpandedSpanIds: Ref<string[]> = ref([]);

// wait for initial render to complete so the transition-all doesn't look glitchy on mount
const isInitialRender = ref(true);
onMounted(() => {
  setTimeout(() => {
    isInitialRender.value = false;
  }, 100);
});

const runTree = new RunTree(props.graph, nodePtr);
const minLevel: Ref<Severity> = ref(Severity.INFO);

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
  span: RunData | SpanData;
  interruption: InterruptionData | null | undefined;
  offsetRelative: number;
  durationRelative: number;
  isTerminal: boolean;
  isBad: boolean;
  isInterrupted: boolean;
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

function makeTimeline(now: DateTime, root: RunData, maxDepth: number | undefined): Timeline {
  const spans: TimelineSpan[] = [];
  const events: TimelineEvent[] = [];

  const nowMs = timestampToMs(now);
  const rootStartedAtMs = getRunStartedAtMs(root);
  const rootDurationMs = getRunDurationMs(root, nowMs);

  function walkRun(run: RunData | SpanData, parent: TimelineSpan | null, depth: number) {
    // timing
    const startedAtMs = getRunStartedAtMs(run);
    const durationMs = getRunDurationMs(run, nowMs);

    // content
    const basePtr = isNode(run, NodeType.RUN) ? getBaseFromNode(run) : null;
    const baseNode = basePtr != null ? props.graph.get(basePtr) : null;
    const color = !isNode(run, NodeType.SPAN)
      ? getColorHex(RunStatusOptionInfo[run.status]!.color!, ColorShade.S500)!
      : getColorHex(ColorType.SUCCESS, ColorShade.S500)!;
    let icon: IconData;
    let name: string = "???";
    if (isNode(run, NodeType.RUN)) {
      icon = (baseNode != null ? getNodeIcon(baseNode) : null) ?? makeIcon(NodeTypeOptionInfo[NodeType.RUN]!.icon!);
      name = (baseNode as any)?.name ?? "Run";
    } else if (isNode(run, NodeType.SPAN)) {
      icon = makeIcon(SpanTypeOptionInfo[run.type]!.icon!);
      name = toCamelName(SpanType, run.type);
    } else {
      assertNever(run);
    }
    const interruption =
      isNode(run, NodeType.RUN) && run.interruptionPtr != null
        ? (runTree.runGraph.get(run.interruptionPtr) as InterruptionData | null)
        : null;

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
      span: run,
      interruption,
      offsetRelative,
      durationRelative: durationRelative,
      isTerminal: isRunTerminal(run),
      isBad: isRunBad(run),
      isInterrupted: isRunInterrupted(run),
    };
    spans.push(span);

    // descend
    if (isNode(run, NodeType.RUN) && (maxDepth == null || depth < maxDepth)) {
      for (const child of runTree.runGraph.getChildren(run)) {
        if (isNode(child, NodeType.RUN)) {
          const basePtr = getBaseFromNode(child);
          if (basePtr != null && !BASE_TYPES.includes(basePtr.nodeType)) continue;
          walkRun(child, span, depth + 1);
        } else if (isNode(child, NodeType.SPAN)) {
          walkRun(child, span, depth + 1);
        }
      }
    }
  }

  walkRun(root, null, 0);

  const descendants = [...spans.slice(1) /* skip root */, ...events];
  descendants.sort((a, b) => a.startedAtMs - b.startedAtMs);
  const hasActive = spans.some((span) => !span.isTerminal);
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
      const maxDepth = props.layout == "linear" ? 1 : undefined;
      timeline.value = makeTimeline(now.value, root, maxDepth);
    } else {
      timeline.value = EMPTY_TIMELINE;
    }
  }
});
</script>
<template>
  <div v-if="layout == 'linear'" ref="containerRef" class="flex w-full flex-1 flex-col">
    <!-- NOTE :UX :Architecture: maybe RunTimeline should be a more general Feed view? -->
    <!-- Linear -->
    <div v-for="thing in timeline.descendants" :key="thing.id" class="w-full">
      <!-- Run -->
      <div v-if="thing.metatype == 'span' && isNode(thing.span, NodeType.RUN)" class="w-full py-0.5">
        <!-- Header -->
        <div class="flex w-full flex-row items-center py-0.5">
          <!-- Node -->
          <button
            class="group/node truncate hover:cursor-pointer"
            @click.stop="isNode(thing.baseNode) && canvas.goToNode(thing.baseNode)"
          >
            <!-- Icon (from Run if active) -->
            <IconInline
              v-if="isRunActive(thing.span)"
              v-bind="makeIcon(RunStatusOptionInfo[thing.span.status]!.icon!)"
              class="mr-1.5 w-5 text-center text-gray-700"
              :class="[thing.span.status == RunStatus.RUNNING ? 'animate-spin' : '']"
            />
            <IconInline v-else v-bind="thing.icon" class="mr-1.5 w-5 text-center text-gray-700" />
            <!-- Name -->
            <span class="truncate font-medium underline-offset-3 group-hover/node:underline">{{ thing.name }}</span>
            <!-- Icon -->
            <IconInline
              v-if="thing.isBad || thing.isInterrupted"
              v-bind="makeIcon(RunStatusOptionInfo[thing.span.status]!.icon!)"
              :style="{ color: getRunColorHex(thing.span.status) }"
              class="ml-1 w-5 text-center"
            />
          </button>
          <!-- Status / Commands -->
          <div class="ml-auto flex flex-row items-center gap-x-1.5">
            <button
              v-if="
                IS_DEVELOPER_MODE && (thing.baseNode as ActionData)?.type != ActionType.CODE && thing.span.code != null
              "
              class="rounded px-0.5 transition-colors duration-150 hover:bg-gray-100"
              :class="[
                codeExpandedSpanIds.includes(thing.id) ? 'text-primary-700' : 'text-gray-400 hover:text-gray-700',
              ]"
              @click="
                codeExpandedSpanIds.includes(thing.id)
                  ? codeExpandedSpanIds.splice(codeExpandedSpanIds.indexOf(thing.id), 1)
                  : codeExpandedSpanIds.push(thing.id)
              "
            >
              <span class="fas fa-code" />
            </button>
            <button
              v-for="command in getRunCommands(thing.span)"
              :key="command.title"
              v-tooltip="{ title: command.title, small: true, group: 'run.header' }"
              class="rounded px-0.5 text-gray-400 transition-colors duration-150 hover:text-gray-700"
              @click="command.command()"
            >
              <IconInline v-bind="command.icon" />
            </button>
            <span class="text-gray-400">
              {{ getRunDurationString(thing.span, { minUnit: "s" }) }}
            </span>
          </div>
        </div>
        <!-- Body -->
        <div class="ml-2 flex flex-row gap-x-3">
          <!-- Connecting line -->
          <div
            class="w-1 rounded-full"
            :class="[thing.isBad ? 'bg-red-500' : thing.isInterrupted ? 'bg-pink-500' : 'bg-gray-200']"
          />
          <!-- Content -->
          <div class="flex flex-1 flex-col pb-1.5 pt-1">
            <!-- Inputs/Outputs -->
            <SomeObject
              v-if="thing.span.inputsPacked != null"
              :id="`object-inputs-${thing.id}`"
              class=""
              :value-type="getInputType(thing.baseNode as RunnableNode)"
              is-inline
              is-minimal
              :model-value="thing.span.inputsPacked"
            />
            <SomeObject
              v-if="thing.span.outputsPacked != null"
              :id="`object-outputs-${thing.id}`"
              class=""
              :value-type="getOutputType(thing.baseNode as RunnableNode)"
              is-inline
              :is-input="thing.interruption != null"
              is-minimal
              :model-value="thing.span.outputsPacked"
            />
            <!-- Model logic for dynamic commands -->
            <div
              v-if="
                IS_DEVELOPER_MODE &&
                (thing.baseNode as any)?.type != ActionType.CODE &&
                (thing.span.code as any) != null &&
                codeExpandedSpanIds.includes(thing.id)
              "
              class="mt-2"
            >
              <div>
                <span class="fas fa-code mr-1.5 text-gray-700" />
                <span class="">Code</span>
              </div>
              <Code id="code" class="mt-1" :model-value="thing.span.code" />
            </div>
            <!-- Interruption -->
            <div v-if="thing.interruption != null" class="">
              <span>The Run was interrupted.</span>
              <div
                v-if="thing.interruption.status == InterruptionStatus.OPEN"
                class="flex flex-row items-center justify-end gap-x-2 py-1"
              >
                <button
                  class="rounded px-0.5 text-pink-700 transition-colors duration-75 hover:bg-gray-100"
                  @click="runtime.complete(thing.interruption)"
                >
                  <i class="fas fa-check" />
                  <span class="ml-1.5">Continue</span>
                </button>
                <button
                  class="rounded px-0.5 text-gray-700 transition-colors duration-75 hover:bg-gray-100 hover:text-gray-900"
                  @click="runtime.cancel(thing.interruption)"
                >
                  <i class="fas fa-xmark" />
                  <span class="ml-1.5">Cancel</span>
                </button>
              </div>
            </div>
            <!-- Error -->
            <Error v-if="thing.span.error != null" :error="thing.span.error" class="mt-1.5" />
          </div>
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
            v-bind="span.icon ?? makeIcon(NodeTypeOptionInfo[NodeType.RUN]!.icon!)"
            class="mr-1.5 w-5 text-center text-gray-700 transition-colors duration-75"
          />
          <span class="truncate underline-offset-3 group-hover/node:underline">{{ span.name }}</span>
        </button>
        <!-- Meta -->
        <div class="ml-auto flex-shrink-0 pl-1.5">
          <RunStatus
            v-if="isNode(span.span, NodeType.RUN)"
            :run="span.span"
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

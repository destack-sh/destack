<script lang="ts" setup>
import {
  AnyNodeData,
  ColorShade,
  IconData,
  NodeType,
  RunAttemptData,
  RunData,
  RunSpanData,
  RunStatus,
  ViewData,
} from "@/proto/wire";
import { isNode, SomeNodeReferenceData, TypedNodeReferenceData } from "@/proto/wiring";
import { RunTree } from "@/system/runtime";
import { canvas, pkgGraph } from "@/system/space";
import { getNodeIcon, ICON_BY_NODE_TYPE, ICON_BY_RUN_STATUS, IconInline } from "@/ui/icon";
import { COLOR_BY_RUN_STATUS, getColorHex } from "@/ui/style";
import { durationToMs, formatDuration, getNow, timestampToMs, TimeUpdateInterval } from "@/utils/time";
import { computed, Ref, toRef } from "vue";

const DEPTH_OFFSET = 12;
const ROW_HEIGHT = 28;
const BAR_PADDING = 4;

const props = defineProps<{ nodePtr: SomeNodeReferenceData } & Pick<ViewData, "size">>();
const nodePtr = toRef(props, "nodePtr") as Ref<TypedNodeReferenceData<NodeType.RUN> | null>;

//
// Run
//

const runTree = new RunTree(pkgGraph, nodePtr);
const run = runTree.runRef;

//
// Spans / Events
//

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

function getDurationMs(run: RunData, nowMs: number): number {
  const startedAtMs = timestampToMs(run.startedAt ?? run.createdAt!);
  let durationMs: number;
  if (run.duration != null) {
    durationMs = durationToMs(run.duration);
  } else {
    const currentMs = run.terminatedAt != null ? timestampToMs(run.terminatedAt) : nowMs;
    durationMs = currentMs - startedAtMs;
  }
  return durationMs;
}

const spans: Ref<TimelineSpan[]> = computed(() => {
  const spans: TimelineSpan[] = [];
  if (runTree.run == null) return spans;

  const now = getNow(TimeUpdateInterval.MILLISECOND).value;
  const nowMs = timestampToMs(now);
  const rootStartedAtMs = getStartedAtMs(runTree.run);
  const rootDurationMs = getDurationMs(runTree.run, nowMs);

  function walkRun(run: RunData, parent: TimelineSpan | null, depth: number) {
    // timing
    const startedAtMs = getStartedAtMs(run);
    const durationMs = getDurationMs(run, nowMs);

    // context
    let baseNode: AnyNodeData | null = null;
    if (run.stepPtr != null) {
      baseNode = pkgGraph.get(run.stepPtr);
    } else if (run.blockPtr != null) {
      baseNode = pkgGraph.get(run.blockPtr);
    }
    const color = getColorHex(COLOR_BY_RUN_STATUS[run.status])!;

    // span
    const widthRelative = Math.max(0, Math.min(1, durationMs / rootDurationMs));
    const offsetRelative = Math.max(0, Math.min(1, (startedAtMs - rootStartedAtMs) / rootDurationMs));
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
    };
    spans.push(span);

    // descend
    for (const child of runTree.runGraph.getChildren(run)) {
      if (!isNode(child, NodeType.RUN)) continue;
      walkRun(child, span, depth + 1);
    }
  }

  walkRun(runTree.run, null, 0);
  return spans;
});

// nocheckin: RunTimeline
</script>
<template>
  <div class="flex w-full flex-row gap-x-2">
    <!-- Timeline -->
    <!-- Spans -->
    <div class="flex flex-1 flex-col">
      <!-- Span -->
      <div
        v-for="span in spans"
        :key="span.id"
        class="group flex w-full flex-row items-center rounded hover:bg-gray-100"
      >
        <!-- Tree  -->
        <div
          class="flex w-[320px] flex-shrink-0 flex-row items-center pr-2"
          :style="{
            paddingLeft: 8 + span.depth * DEPTH_OFFSET + 'px',
          }"
        >
          <!-- Node -->
          <span class="hover:cursor-pointer" @click="isNode(span.baseNode) && canvas.goToNode(span.baseNode)">
            <IconInline
              v-bind="span.icon ?? ICON_BY_NODE_TYPE[NodeType.RUN]"
              class="mr-1 w-5 text-center transition-colors duration-75"
            />
            <span>{{ span.title }}</span>
          </span>
          <!-- Meta -->
          <div class="ml-auto">
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
          class="transform rounded transition-all duration-150"
          :style="{
            height: ROW_HEIGHT - 2 * BAR_PADDING + 'px',
            marginTop: BAR_PADDING + 'px',
            marginBottom: BAR_PADDING + 'px',
            width: span.widthRelative * 100 + '%',
            marginLeft: span.offsetRelative * 100 + '%',
            backgroundColor: span.color,
          }"
        ></div>
      </div>
    </div>
  </div>
</template>

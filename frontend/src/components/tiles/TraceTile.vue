<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import RunCacheInfo from "@/components/tiles/RunCacheInfo.vue";
import RunTile from "@/components/tiles/RunTile.vue";
import { pinAbsoluteElement } from "@/composables/useFixed";
import { formatDuration, useNow } from "@/composables/useNow";
import { RunStatus, StatementType, type Run, type Statement } from "@/gql/graphql";
import { useBenchState } from "@/state/bench";
import { useCurrentModule, useNavigation } from "@/state/module";
import { TERMINAL_RUN_STATUSES, getRunStatusColor, getRunStatusIconSolid, useRun } from "@/state/session";
import { getUUIDFromGlobalID } from "@/utils/functools";
import { useElementBounding, useKeyModifier } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, ref, toRef, type Ref, watch } from "vue";

type TRACE_LAYOUT = "list" | "bars" | "table";

const props = defineProps<{
  sessionId?: string;
  rootId: string;
  layout: TRACE_LAYOUT;
  filter?: {
    statuses?: RunStatus[];
  };
  live?: boolean;
}>();

const layout = ref<TRACE_LAYOUT>(props.layout);
const bench = useBenchState();
const module = useCurrentModule();
const nav = useNavigation();
const now = useNow(100);
const altKey = useKeyModifier("Alt");

// sync layout from props on change
watch(
  () => props.layout,
  () => {
    layout.value = props.layout;
  }
);

const canvasRef: Ref<HTMLDivElement | null> = ref(null);
const canvasBounding = useElementBounding(canvasRef);

const focusedRunPopoverRef: Ref<HTMLDivElement | null> = ref(null);
const focusedNode = ref<OrderedNode | BarNode | null>(null);
const focusedRunPin = pinAbsoluteElement(focusedRunPopoverRef, { pos: true, keepInView: true });

type OrderedNode = {
  id: string;
  run: Run;
  terminated: boolean;
  duration: number;
  durationFraction: number;
  durationSelf: number;
  durationSelfFraction: number;
  runnable?: Statement;
  ancestors: OrderedNode[];
  children: OrderedNode[];
  depth: number;
};

// run trace
const { run: root, loading, childrenByParentId } = useRun(toRef(props, "rootId"), { live: props.live });

const orderedNodes: Ref<OrderedNode[]> = computed(() => {
  // walk the tree and position nodes using children (by parent)
  if (root.value == null) return [];
  const orderedNodes: OrderedNode[] = [];

  function _walk(run: Run, ancestors: OrderedNode[]): OrderedNode {
    const runnable = module.statementOf(run.runnable?.id);
    const terminated = TERMINAL_RUN_STATUSES.includes(run.status);
    const duration = terminated
      ? run.duration ?? 0
      : now.value.diff(DateTime.fromISO(run.startedAt ?? run.createdAt), "seconds").seconds;
    const rootDuration = ancestors[0]?.duration ?? duration;
    const node = {
      id: run.id,
      run,
      terminated,
      duration,
      durationFraction: duration / rootDuration,
      runnable,
      ancestors,
      depth: ancestors.length,
      children: [] as OrderedNode[],
    } as OrderedNode;

    if (node.runnable == null) {
      return node; // ignore because non-symbolx.lib dependencies are not loaded yet
    }
    orderedNodes.push(node);

    // walk children
    const children = childrenByParentId.value[run.id]?.slice() ?? [];
    children.sort((a, b) => a.createdAt.localeCompare(b.createdAt));
    ancestors = [...ancestors, node];
    for (const child of children) {
      const childNode = _walk(child, ancestors);
      node.children.push(childNode);
    }
    const durationChildren = node.children.reduce((acc, child) => acc + child.duration, 0);
    node.durationSelf = node.duration - durationChildren;
    node.durationSelfFraction = node.durationSelf / node.duration;
    return node;
  }

  _walk(root.value as Run, []);

  return orderedNodes;
});

type BarNode = OrderedNode & {
  width: number;
  x: number;
  y: number;
  color: string;
};
const barPaddingX = 0;
const barHeight = 32;
const barGapY = 2;
const barMinWidth = 4;

const bgColorByNodeType: Partial<Record<StatementType, string>> = {
  [StatementType.Model]: "bg-teal-100 border-teal-200",
  [StatementType.Code]: "bg-blue-100 border-blue-200",
  [StatementType.Task]: "bg-orange-100 border-orange-200",
};

const bars = computed(() => {
  const root = orderedNodes.value[0];
  if (root == null) return [];
  const bars: BarNode[] = [];
  const targetWidth = canvasBounding.width.value - 2 * barPaddingX;
  const rootStart = DateTime.fromISO(root.run.startedAt);

  // time on x, depth on y
  for (const node of orderedNodes.value) {
    const durationFraction = node.duration / root.duration;
    const width = Math.max(barMinWidth, Math.round(durationFraction * targetWidth));
    const timeOffset = DateTime.fromISO(node.run.startedAt).diff(rootStart, "seconds").seconds;
    const x = Math.round((timeOffset / root.duration) * targetWidth);
    const y = node.depth * (barHeight + barGapY);
    const color = bgColorByNodeType[node.runnable?.type ?? StatementType.Code] ?? "bg-gray-600";
    bars.push({ ...node, width, x, y, color });
  }

  return bars;
});
const totalHeight = computed(() => bars.value.reduce((a, b) => Math.max(a, b.y + barHeight), 0));

function getAbsoluteNodePosition(node: OrderedNode | BarNode): { top: string; left: string } {
  if (layout.value == "bars") {
    return {
      top: (node as BarNode).y + "px",
      left: (node as BarNode).x + "px",
    };
  } else if (layout.value == "list") {
    // it's laid out linearly, so just use the offset * 20px
    const idx = orderedNodes.value.findIndex((n) => n.id == node.id);
    return {
      top: 32 * idx + "px",
      left: node.depth * 20 + "px",
    };
  } else {
    throw new Error(`unexpected layout: ${layout.value}`);
  }
}

// other traces will come later (timeline, mutations, logs, etc.)
</script>
<template>
  <div ref="canvasRef" class="relative w-full">
    <div v-if="loading" class="w-full text-center">
      <BusySpinnerIcon class="h-4 w-4 animate-spin text-gray-400" />
    </div>
    <div v-else-if="orderedNodes.length == 0" class="w-full text-center">
      <span class="text-gray-400">No trace</span>
    </div>
    <div v-else-if="layout == 'list'" class="flex h-full w-full flex-col gap-0.5">
      <!-- Run tree -->
      <div
        v-for="node in orderedNodes"
        :key="node.id"
        class="flex flex-row items-center rounded-sm p-1 hover:cursor-pointer hover:bg-orange-100"
        :class="[
          getRunStatusColor(node.run.status),
          focusedNode?.id == node.id ? 'ring-inset-1 ring-1 ring-orange-600 ring-opacity-40' : '',
        ]"
        :style="{
          marginLeft: node.depth * 20 + 'px',
        }"
        @click="focusedNode = node"
      >
        <!-- Status -->
        <component
          :is="getRunStatusIconSolid(node.run.status)"
          class="h-4 w-4"
          :class="[getRunStatusIconSolid(node.run.status) == BusySpinnerIcon ? 'animate-spin' : '']"
        />
        <!-- Runnable -->
        <span
          class="ml-1 max-w-full truncate font-semibold underline-offset-4"
          :class="[altKey && node.runnable != null ? 'cursor-pointer hover:underline' : '']"
          @click="
            (e) =>
              altKey && node.runnable != null
                ? (nav.focusStatement(node.runnable), e.stopPropagation(), e.preventDefault())
                : undefined
          "
          >{{ node.runnable?.name ?? "???" }}</span
        >
        <!-- Duration -->
        <span class="ml-1">
          <span class="font-semibold">{{ formatDuration(node.duration * 1000) }}</span>
          <template v-if="node.children.length > 0">
            /
            <span class="font-light">{{ formatDuration(node.durationSelf * 1000) }}</span>
          </template>
        </span>
      </div>
    </div>
    <div
      v-else-if="layout == 'bars'"
      class="relative w-full"
      :style="{
        height: totalHeight + 'px',
      }"
    >
      <div
        v-for="node in bars"
        :key="node.id"
        class="absolute flex max-h-full max-w-full flex-row items-center truncate rounded-sm border border-opacity-60 bg-opacity-60 p-1 transition-all hover:z-10 hover:min-w-fit hover:cursor-pointer hover:border-opacity-100 hover:bg-opacity-100"
        :class="[node.color, focusedNode?.id == node.id ? 'ring-1 ring-inset ring-orange-600 ring-opacity-40' : '']"
        :style="{
          left: node.x + 'px',
          top: node.y + 'px',
          width: node.width + 'px',
          height: barHeight + 'px',
        }"
        @click="focusedNode = node"
      >
        <!-- Status -->
        <component
          :is="getRunStatusIconSolid(node.run.status)"
          class="h-4 w-4 flex-shrink-0"
          :class="[
            node.run.status == RunStatus.Running || node.run.status == RunStatus.Queued ? 'animate-spin' : '',
            getRunStatusColor(node.run.status),
          ]"
        />
        <!-- Runnable -->
        <span
          class="ml-1 max-w-full flex-shrink-0 whitespace-nowrap font-semibold underline-offset-4"
          :class="[
            altKey && node.runnable != null ? 'cursor-pointer hover:underline' : '',
            getRunStatusColor(node.run.status),
          ]"
          @click="
            (e) =>
              altKey && node.runnable != null
                ? (nav.focusStatement(node.runnable), e.stopPropagation(), e.preventDefault())
                : undefined
          "
          >{{ node.runnable?.name ?? "???" }}</span
        >
        <!-- Duration -->
        <span
          class="ml-1 flex flex-shrink-0 flex-row flex-nowrap items-center"
          :class="[getRunStatusColor(node.run.status)]"
        >
          <span class="font-semibold">{{ formatDuration(node.duration * 1000) }}</span>
          <template v-if="node.children.length > 0">
            /
            <span class="font-light">{{ formatDuration(node.durationSelf * 1000) }}</span>
          </template>
          <RunCacheInfo :run="node.run" class="px-1" />
        </span>
      </div>
    </div>
    <!-- Prevent scroll and capture click outside -->
    <div
      v-if="focusedRunPopoverRef != null"
      class="fixed left-0 top-0 z-40 h-full w-full overscroll-none"
      @click.stop="focusedNode = null"
    />
    <!-- Focused node -->
    <FadeTransition>
      <div
        v-if="focusedNode != null"
        ref="focusedRunPopoverRef"
        class="z-50 flex w-[400px] flex-col gap-2 rounded-sm bg-white p-2 text-gray-900 shadow-md ring-1 ring-orange-900 ring-opacity-40"
        :style="focusedRunPin.pinned.value ? {} : getAbsoluteNodePosition(focusedNode)"
        :class="[focusedRunPin.pinned.value ? '' : 'absolute']"
      >
        <!-- Runnable -->
        <div class="flex flex-row" v-if="focusedNode.runnable != null">
          <a
            class="cursor-pointer font-semibold underline-offset-2 hover:underline"
            @click="bench.openRun(focusedNode, { focus: true })"
          >
            {{ focusedNode?.runnable?.name }} #{{ getUUIDFromGlobalID(focusedNode.runnable.id).slice(-7, -1) }}
          </a>
        </div>
        <RunTile
          :run="focusedNode.run"
          :project-version-id="module.id.value"
          :project-id="(bench.projectId as string)"
          show-controls
          view="logs"
        />
      </div>
    </FadeTransition>
  </div>
</template>

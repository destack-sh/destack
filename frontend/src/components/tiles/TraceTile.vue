<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import { formatDurationSeconds, useNow } from "@/composables/useNow";
import { RunStatus, StatementType, type Run, type Statement } from "@/gql/graphql";
import { useCurrentModule, useNavigation } from "@/state/module";
import { RUN_TERMINAL_STATES } from "@/state/session";
import { getStatusColor, getStatusIconSolid, useRun } from "@/state/session";
import { useElementBounding, useKeyModifier } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, ref, toRef, type Ref } from "vue";

type TRACE_LAYOUT = "list" | "bartree" | "table";

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
const module = useCurrentModule();
const nav = useNavigation();
const now = useNow(100);
const canvasRef: Ref<HTMLDivElement | null> = ref(null);
const canvasBounding = useElementBounding(canvasRef);
const altKey = useKeyModifier("Alt");

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
const { run: root, loading, nodes, childrenByParentId } = useRun(toRef(props, "rootId"), { live: props.live });

const orderedNodes: Ref<OrderedNode[]> = computed(() => {
  // walk the tree and position nodes using children (by parent)
  if (root.value == null) return [];
  const orderedNodes: OrderedNode[] = [];

  function _walk(run: Run, ancestors: OrderedNode[]): OrderedNode {
    const runnable = module.statementOf(run.runnable?.id);
    const terminated = RUN_TERMINAL_STATES.includes(run.status);
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
const barPaddingY = 4;
const barHeight = 20;
const barGapY = 2;
const barMinWidth = 4;

const bgColorByNodeType = {
  [StatementType.Model]: "bg-teal-600",
  [StatementType.Code]: "bg-blue-600",
  [StatementType.Task]: "bg-orange-600",
};

const bars = computed(() => {
  const root = orderedNodes.value[0];
  if (root == null) return [];
  const bars: BarNode[] = [];
  const targetWidth = canvasBounding.width.value - 2 * barPaddingX;
  const rootStart = DateTime.fromISO(root.run.createdAt);

  // time on x, depth on y
  for (const node of orderedNodes.value) {
    const durationFraction = node.duration / root.duration;
    const width = Math.max(barMinWidth, Math.round(durationFraction * targetWidth));
    const timeOffset = DateTime.fromISO(node.run.createdAt).diff(rootStart, "seconds").seconds;
    const x = Math.round((timeOffset / root.duration) * targetWidth);
    const y = node.depth * (barHeight + barGapY);
    const color = bgColorByNodeType[node.runnable?.type] ?? "bg-gray-600";
    bars.push({ ...node, width, x, y, color });
  }

  return bars;
});

// other traces will come later (timeline, mutations, logs, etc.)
</script>
<template>
  <div v-if="loading" class="w-full text-center">
    <BusySpinnerIcon class="h-4 w-4 animate-spin text-gray-400" />
  </div>
  <div ref="canvasRef" v-else-if="layout == 'list'" class="flex h-full w-full flex-col gap-0.5">
    <!-- Run tree -->
    <div
      v-for="node in orderedNodes"
      :key="node.id"
      class="flex flex-row items-center rounded-sm p-1 hover:bg-orange-100"
      :class="[getStatusColor(node.run.status)]"
      :style="{
        marginLeft: node.depth * 20 + 'px',
      }"
    >
      <!-- Status -->
      <component
        :is="getStatusIconSolid(node.run.status)"
        class="h-4 w-4"
        :class="[node.run.status == RunStatus.Running || node.run.status == RunStatus.Queued ? 'animate-spin' : '']"
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
        <span class="font-semibold">{{ formatDurationSeconds(node.duration * 1000) }}</span>
        <template v-if="node.children.length > 0">
          /
          <span class="font-light">{{ formatDurationSeconds(node.durationSelf * 1000) }}</span>
        </template>
      </span>
    </div>
    <!-- nocheckin -->
  </div>
  <div ref="canvasRef" v-else-if="layout == 'bartree'" class="relative h-full w-full">
    <div
      v-for="bar in bars"
      :key="bar.id"
      class="absolute flex max-h-full max-w-full flex-row items-center rounded-sm p-1 hover:bg-orange-100"
      :class="[getStatusColor(bar.run.status)]"
      :style="{
        left: bar.x + 'px',
        top: bar.y + 'px',
        width: bar.width + 'px',
        height: barHeight + 'px',
      }"
    >
      <!-- Status -->
      <component
        :is="getStatusIconSolid(bar.run.status)"
        class="h-4 w-4"
        :class="[bar.run.status == RunStatus.Running || bar.run.status == RunStatus.Queued ? 'animate-spin' : '']"
      />
      <!-- Runnable -->
      <span
        class="ml-1 max-w-full truncate font-semibold underline-offset-4"
        :class="[altKey && bar.runnable != null ? 'cursor-pointer hover:underline' : '']"
        @click="
          (e) =>
            altKey && bar.runnable != null
              ? (nav.focusStatement(bar.runnable), e.stopPropagation(), e.preventDefault())
              : undefined
        "
        >{{ bar.runnable?.name ?? "???" }}</span
      >
    </div>
  </div>
</template>

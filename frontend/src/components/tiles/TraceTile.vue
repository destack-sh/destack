<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import { formatDurationSeconds, useNow } from "@/composables/useNow";
import { RunStatus, type Run, type Statement } from "@/gql/graphql";
import { useCurrentModule, useNavigation } from "@/state/module";
import { getStatusColor, getStatusIconSolid, useRun } from "@/state/session";
import { useElementBounding, useKeyModifier } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, ref, toRef, type Ref } from "vue";

type TRACE_LAYOUT = "list" | "bartree" | "table";

const props = defineProps<{
  sessionId?: string;
  rootId: string;
  layout?: TRACE_LAYOUT;
  filter?: {
    statuses?: RunStatus[];
  };
  live?: boolean;
}>();

const layout = ref<TRACE_LAYOUT>(props.layout ?? "list");
const module = useCurrentModule();
const nav = useNavigation();
const now = useNow(100);
const canvasRef: Ref<HTMLDivElement | null> = ref(null);
const canvasBounding = useElementBounding(canvasRef);
const altKey = useKeyModifier("Alt");

type OrderedNode = {
  id: string;
  run: Run;
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
    const duration = run.duration ?? DateTime.fromISO(run.startedAt ?? run.createdAt).diff(now.value).seconds;
    const rootDuration = ancestors[0]?.duration ?? duration;
    const node = {
      id: run.id,
      run,
      duration,
      durationFraction: duration / rootDuration,
      runnable,
      ancestors,
      depth: ancestors.length,
      children: [] as OrderedNode[],
    } as OrderedNode;
    orderedNodes.push(node);

    // walk children
    const children = childrenByParentId.value[run.id]?.slice() ?? [];
    children.sort((a, b) => a.createdAt.localeCompare(b.createdAt));
    ancestors = [...ancestors, node];
    for (const child of children) {
      node.children.push(_walk(child, ancestors));
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

const bars = computed(() => {
  const bars: BarNode[] = [];
  // nocheckin: do this
  return bars;
});

// other traces will come later (timeline, mutations, logs, etc.)
</script>
<template>
  <div v-if="loading" class="w-full text-center">
    <BusySpinnerIcon class="h-4 w-4 animate-spin text-gray-400" />
  </div>
  <div ref="canvasRef" v-else-if="layout == 'list'" class="flex h-full w-full flex-col gap-0.5">
    <!-- Node tree -->
    <div
      v-for="node in orderedNodes"
      :key="node.id"
      class="flex flex-row items-center rounded-sm p-0.5 hover:bg-orange-100"
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
  <div ref="canvasRef" v-else-if="layout == 'bartree'" class="h-full w-full">
    I'm a tree with {{ nodes?.length }} bars
    <!-- nocheckin -->
  </div>
</template>

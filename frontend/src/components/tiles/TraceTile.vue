<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import RunCacheInfo from "@/components/tiles/RunCacheInfo.vue";
import { formatDuration, useNow } from "@/composables/useNow";
import type { RunStatus, Run, Statement } from "@/gql/graphql";
import { useBenchState, usePanelContext } from "@/state/bench";
import { useCurrentModule, useNavigation } from "@/state/module";
import { TERMINAL_RUN_STATUSES, getRunStatusColor, getRunStatusIconSolid, useRun } from "@/state/session";
import { getUUIDFromGlobalID } from "@/utils/functools";
import { useElementBounding, useKeyModifier } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, ref, toRef, type Ref, watch } from "vue";

type TRACE_LAYOUT = "list";

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
const panel = usePanelContext();
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

type OrderedNode = {
  id: string;
  run: Run;
  terminated: boolean;
  duration: number;
  durationFraction: number;
  durationSelf: number;
  durationSelfFraction: number;
  statement?: Statement;
  ancestors: OrderedNode[];
  children: OrderedNode[];
  depth: number;
  // custom metadata
  name?: string;
  retry?: number;
  test?: boolean;
  nonce?: number;
};

// run trace
const { run: root, loading, nodes, childrenByParentId } = useRun(toRef(props, "rootId"), { live: props.live });

const orderedNodes: Ref<OrderedNode[]> = computed(() => {
  // walk the tree and position nodes using children (by parent)
  if (root.value == null) return [];
  const orderedNodes: OrderedNode[] = [];

  const metaNameKey = module.runMetadataKey("name") ?? "";
  const metaRetryKey = module.runMetadataKey("retry") ?? "";
  const metaTestKey = module.runMetadataKey("test") ?? "";
  const metaNonceKey = module.runMetadataKey("nonce") ?? "";

  function _walk(run: Run, ancestors: OrderedNode[]): OrderedNode {
    const statement = module.statementOf(run.statementCk);
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
      statement,
      ancestors,
      depth: ancestors.length,
      children: [] as OrderedNode[],
      // custom metadata
      name: run.value?.[metaNameKey] ?? undefined,
      retry: run.value?.[metaRetryKey] ?? undefined,
      test: run.value?.[metaTestKey] ?? undefined,
      nonce: run.value?.[metaNonceKey] ?? undefined,
    } as OrderedNode;

    orderedNodes.push(node);

    // walk children
    const children = childrenByParentId.value[run.id]?.slice() ?? [];
    children.sort((a, b) => (a.startedAt ?? a.createdAt).localeCompare(b.startedAt ?? b.createdAt));
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

function openRun(run: Run) {
  bench.openViewRun(run, { focus: true, group: panel.panel.value.group });
}

// other traces will come later (timeline, mutations, logs, etc.)
</script>
<template>
  <div ref="canvasRef" class="relative w-full">
    <div v-if="loading" class="flex w-full flex-row justify-center text-center">
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
        class="group/run rounded-sm p-1 hover:cursor-pointer hover:bg-orange-100"
        :class="[getRunStatusColor(node.run.status)]"
        :style="{
          marginLeft: node.depth * 20 + 'px',
        }"
        @click.stop="openRun(node.run)"
      >
        <!-- Header -->
        <div class="flex flex-row items-center">
          <!-- Status -->
          <component
            :is="getRunStatusIconSolid(node.run.status)"
            class="h-4 w-4"
            :class="[getRunStatusIconSolid(node.run.status) == BusySpinnerIcon ? 'animate-spin' : '']"
          />
          <!-- Runnable -->
          <span class="ml-1 max-w-full truncate font-semibold">
            {{ node.name ?? node.statement?.name ?? "???" }}
          </span>
          <!-- Duration -->
          <span class="ml-1 flex flex-row items-center">
            <span class="font-semibold">{{ formatDuration(node.duration * 1000) }}</span>
            <template v-if="node.children.length > 0">
              /
              <span class="font-light">{{ formatDuration(node.durationSelf * 1000) }}</span>
            </template>
            <RunCacheInfo :run="node.run" class="px-0.5" />
          </span>
          <!-- Run id -->
          <span class="ml-1 font-normal text-gray-400 underline-offset-4 group-hover/run:underline">
            #{{ getUUIDFromGlobalID(node.id).slice(-7, -1) }}
          </span>
        </div>
        <!-- Extra metadata -->
        <!-- not sure what to show here yet -->
        <!-- <div v-if="node.retry != null" class="ml-5 flex flex-row items-center gap-0.5">
          <span class="text-xs text-gray-400">retry: {{ node.retry }}</span>

        </div> -->
      </div>
    </div>
  </div>
</template>

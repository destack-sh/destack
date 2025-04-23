<script lang="ts" setup>
import { ACTIVE_PROCESS_STATUSES, INTERRUPTED_PROCESS_STATUSES, toCamelName } from "@/language/core/const";
import { makeExpression } from "@/language/core/expression";
import { isProcessActive } from "@/language/runtime/process";
import {
  AnyNodeData,
  ColorShade,
  ExpressionType,
  NodeType,
  ProcessStatusOptionInfo,
  TextLineType,
  ThreadData,
  ThreadProperty,
  ViewData,
} from "@/proto/wire";
import { describeNode, propertyReference, TypedNodeReferenceData } from "@/proto/wiring";
import { CURRENT_BENCH_SCOPE } from "@/system/client";
import { useSearchConnection } from "@/system/connection";
import { canvas, threadPtr } from "@/system/space";
import { CONTEXT_COMMANDS_BY_TYPE, fireCommandById, getCommand } from "@/ui/command";
import { startSelectingIfAllowed, useSelectionZone } from "@/ui/drag";
import { getNodeIcon, IconInline } from "@/ui/icon";
import { getColorHex } from "@/ui/style";
import { VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import { getNow, TimeUpdateInterval } from "@/utils/time";
import TextLine from "@/views/content/TextLine.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import SelectionOverlay from "@/views/overlays/SelectionOverlay.vue";
import { DateTime } from "luxon";
import { computed, Ref, ref, toRef } from "vue";

const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const ITEM_HEIGHT = 30;
const DEFAULT_SORT = [
  makeExpression({
    type: ExpressionType.DESCENDING,
    propertyPtr: propertyReference(NodeType.THREAD, ThreadProperty.createdAt),
  }),
];
const NODE_COMMANDS = CONTEXT_COMMANDS_BY_TYPE[NodeType.THREAD]?.map(getCommand);

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
  } & Partial<Pick<ViewData, "subnodePacked">>
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");

// search
const {
  connection,
  graph,
  page,
  roots: threads,
  isConnecting,
  isStale,
} = useSearchConnection(
  { name: "list", live: true },
  computed(() => ({
    nodeType: NodeType.THREAD,
    scope: CURRENT_BENCH_SCOPE.value,
    sort: DEFAULT_SORT,
    isEnabled: true,
    // TODO :Performance: limit & paginate ThreadList?
  })),
);
const total = computed(() => page.value?.total);

// organization (by time)
type ThreadGroup = {
  title: string;
  threads: ThreadData[];
  isFirst: boolean;
  startTime: DateTime;
  endTime: DateTime;
};

const now = getNow(TimeUpdateInterval.HOUR);

// Function to get the active/updated time from a thread
function getThreadTime(thread: ThreadData): DateTime {
  // Use activeAt or createdAt as the time to group by
  if (thread.activeAt) {
    return DateTime.fromMillis(Number(thread.activeAt.seconds) * 1000 + thread.activeAt.nanos / 1000000);
  } else {
    if (!thread.createdAt) {
      throw new Error(`missing createdAt for ${describeNode(thread)}`);
    }
    return DateTime.fromMillis(Number(thread.createdAt.seconds) * 1000 + thread.createdAt.nanos / 1000000);
  }
}

const threadGroups = computed(() => {
  const groups: ThreadGroup[] = [];
  const today = now.value.startOf("day");
  const yesterday = today.minus({ days: 1 });
  const thisWeekStart = today.startOf("week");
  const lastWeekStart = thisWeekStart.minus({ weeks: 1 });

  // hardcoded groups
  const todayGroup: ThreadGroup = {
    title: "Today",
    threads: [],
    isFirst: true,
    startTime: today,
    endTime: today.endOf("day"),
  };
  const yesterdayGroup: ThreadGroup = {
    title: "Yesterday",
    threads: [],
    isFirst: false,
    startTime: yesterday,
    endTime: yesterday.endOf("day"),
  };
  const thisWeekGroup: ThreadGroup = {
    title: "This Week",
    threads: [],
    isFirst: false,
    startTime: thisWeekStart,
    endTime: today.minus({ days: 1 }).endOf("day"),
  };
  const lastWeekGroup: ThreadGroup = {
    title: "Last Week",
    threads: [],
    isFirst: false,
    startTime: lastWeekStart,
    endTime: thisWeekStart.minus({ days: 1 }).endOf("day"),
  };

  // earlier weeks
  const earlierWeeksByStart = new Map<string, ThreadGroup>();
  for (const thread of threads.value) {
    const threadTime = getThreadTime(thread);

    if (threadTime >= today) {
      todayGroup.threads.push(thread);
    } else if (threadTime >= yesterday) {
      yesterdayGroup.threads.push(thread);
    } else if (threadTime >= thisWeekStart) {
      thisWeekGroup.threads.push(thread);
    } else if (threadTime >= lastWeekStart) {
      lastWeekGroup.threads.push(thread);
    } else {
      // for earlier weeks, group by week
      const weekStart = threadTime.startOf("week");
      const weekEnd = weekStart.endOf("week");
      const weekKey = weekStart.toFormat("yyyy-MM-dd");
      if (!earlierWeeksByStart.has(weekKey)) {
        earlierWeeksByStart.set(weekKey, {
          isFirst: false,
          title: `${weekStart.toFormat("LLLL d")} - ${weekEnd.toFormat("LLLL d")}`,
          threads: [thread],
          startTime: weekStart,
          endTime: weekEnd,
        });
      } else {
        earlierWeeksByStart.get(weekKey)!.threads.push(thread);
      }
    }
  }

  // assemble
  groups.push(todayGroup);
  if (yesterdayGroup.threads.length > 0) {
    groups.push(yesterdayGroup);
  }
  if (thisWeekGroup.threads.length > 0) {
    groups.push(thisWeekGroup);
  }
  if (lastWeekGroup.threads.length > 0) {
    groups.push(lastWeekGroup);
  }
  const earlierWeeks = Array.from(earlierWeeksByStart.values()).sort(
    (a, b) => b.startTime.toMillis() - a.startTime.toMillis(),
  );
  groups.push(...earlierWeeks);
  return groups;
});

// interaction
const containerRef = ref<HTMLElement | null>(null);
const itemRefs = ref<Record<string, HTMLElement>>({});
const selectionOverlayRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionZone = useSelectionZone({ containerEl: containerRef, overlayEl: selectionOverlayRef });

// TODO :Robustness: remove & restore Threads breaks some stuff (like it won't respond anymore) :RichGraph
//  (presumably because the search & getr connections get confused.. maybe we should auto-close the thread for now?)

defineExpose<Omit<ViewExpose, "id"> & { total: Ref<number | undefined>; roots: Ref<AnyNodeData[]> }>({
  self,
  total,
  roots: threads,
});
</script>
<template>
  <div ref="containerRef" @mousedown="(e) => startSelectingIfAllowed(selectionZone, e)">
    <div class="relative flex flex-col">
      <!-- Thread groups -->
      <div v-for="group in threadGroups" :key="group.title" class="group/thread-group mb-2">
        <!-- Group header -->
        <div class="group/header mx-3 my-1 flex flex-row items-center px-1.5 py-0.5 text-sm">
          <span class="font-medium">{{ group.title }}</span>
          <!-- Commands -->
          <button
            v-tooltip="{
              small: true,
              text: 'Create Thread',
              shortcuts: getCommand('space.create.thread').shortcuts,
            }"
            class="ml-auto cursor-pointer rounded-sm text-gray-400 opacity-0 transition-colors duration-75 group-hover/thread-group:opacity-100 hover:bg-gray-100 hover:text-gray-700"
            @click="fireCommandById('space.create.thread')"
          >
            <span class="fas fa-plus w-5 text-center" />
          </button>
        </div>

        <!-- Empty state for Today -->
        <div
          v-if="group.title === 'Today' && group.threads.length === 0"
          class="mx-2 flex flex-row items-center rounded-sm px-2 text-sm text-gray-400 transition-colors duration-75 hover:cursor-pointer hover:bg-gray-100"
          role="button"
          :style="{
            height: `${ITEM_HEIGHT}px`,
          }"
          @click="fireCommandById('space.create.thread')"
        >
          <span class="fas fa-plus mr-1 w-5 text-center" />
          <span>Thread</span>
        </div>

        <!-- Threads -->
        <ul class="relative flex flex-col">
          <li
            v-for="thread in group.threads"
            :ref="(ref?: any) => (ref != null ? (itemRefs[thread.id] = ref) : delete itemRefs[thread.id])"
            :key="thread.id"
            class="group/node relative mx-3 flex max-w-full flex-row items-center rounded-sm px-1.5 transition-colors duration-75 hover:cursor-pointer"
            :class="[
              canvas.isSelected(thread)
                ? 'bg-amber-400/20'
                : canvas.isHighlighted(thread)
                  ? 'bg-gray-100'
                  : 'hover:bg-gray-100',
              threadPtr?.id == thread.id ? 'bg-gray-100' : '',
            ]"
            :data-node-type="thread.metatype"
            :data-node-id="thread.id"
            :data-node-ck="(thread as any).ck"
            :data-node-bench-id="(thread as any).benchPtr?.id"
            :style="{
              height: ITEM_HEIGHT + 'px',
            }"
            data-suppress-drag="select"
            role="button"
            @click.stop="canvas.goToNode(thread)"
          >
            <!-- Icon -->
            <IconInline
              v-bind="getNodeIcon(thread)"
              class="mr-1 w-5 text-center text-gray-700 transition-colors duration-75"
            />
            <!-- Name -->
            <TextLine
              :model-value="(thread as any).title"
              :force-line-type="TextLineType.PARAGRAPH"
              class="max-w-full truncate select-none"
              truncate
              is-small
              :placeholder="toCamelName(NodeType, thread.metatype)"
            />
            <!-- Meta -->
            <div class="ml-auto flex flex-row gap-x-1">
              <!-- Metadata -->
              <!-- Run metadata -->
              <span
                class="fas fa-circle-small relative w-5 text-center transition-colors duration-75"
                :class="[
                  ACTIVE_PROCESS_STATUSES.includes(thread.status) ||
                  INTERRUPTED_PROCESS_STATUSES.includes(thread.status)
                    ? 'opacity-100'
                    : 'opacity-0',
                ]"
                :style="{ color: getColorHex(ProcessStatusOptionInfo[thread.status]!.color!, ColorShade.S500) }"
              >
                <span v-if="isProcessActive(thread)" class="fas fa-circle-small absolute inset-0 animate-ping" />
              </span>
            </div>
          </li>
        </ul>
      </div>

      <!-- Selection overlay -->
      <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
    </div>
  </div>
</template>

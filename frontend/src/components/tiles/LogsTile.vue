<script lang="ts" setup>
import type { LogEntry, Conditional, Sort } from "@/gql/graphql";
import { useLogs } from "@/state/session";
import { ChevronDoubleDownIcon } from "@heroicons/vue/24/outline";
import { useElementBounding, useScroll } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, ref, toRef, watch, watchEffect } from "vue";

const props = defineProps<{
  projectId: string;
  projectVersionId: string;
  statementIds?: string[];
  statementCks?: string[];
  runId?: string;
  sessionId?: string;
  query?: Conditional;
  sort?: [Sort];
  live?: boolean;
  limit?: number;
  focus?: {
    statementIds?: string[];
    statementCks?: string[];
    runId?: string;
    sessionId?: string;
  };
  highlight?: boolean;
  lowlight?: boolean;
  containerHeight?: number;
  hideIfEmpty?: boolean;
  hideMetadata?: boolean;
  alwaysExpand?: boolean;
}>();

const { logs, loading, addLogs } = useLogs(
  {
    projectId: toRef(props, "projectId"),
    projectVersionId: toRef(props, "projectVersionId"),
    statementIds: toRef(props, "statementIds"),
    statementCks: toRef(props, "statementCks"),
    sessionId: toRef(props, "sessionId"),
    runId: toRef(props, "runId"),
    query: toRef(props, "query"),
    sort: toRef(props, "sort"),
    after: ref(null),
  },
  {
    live: props.live,
    count: true,
    limit: props.limit,
  }
);
const logsSorted = computed(() => logs.value?.slice().sort((a, b) => a.createdAt.localeCompare(b.createdAt)) ?? []);

function isHighlighted(log: LogEntry): boolean {
  return (
    hasFocus.value &&
    (props.focus?.statementIds == null || props.focus.statementIds.includes(log.statementId)) &&
    (props.focus?.statementCks == null || props.focus.statementCks.includes(log.statementCk)) &&
    (props.focus?.runId == null || props.focus.runId == log.runId) &&
    (props.focus?.sessionId == null || props.focus.sessionId == log.sessionId)
  );
}
const containerRef = ref<HTMLDivElement | null>(null);
const containerScroll = useScroll(containerRef);
const containerBounding = useElementBounding(containerRef);
const logsRef = ref<HTMLDivElement | null>(null);
const logsBounding = useElementBounding(logsRef);
const hasFocus = computed(() => props.focus != null);
const autoscroll = ref(true);
const lastScrollY = ref(0);
const expandedLogs = ref<string[]>([]);

function toggleExpanded(log: LogEntry) {
  if (expandedLogs.value.includes(log.id)) {
    expandedLogs.value = expandedLogs.value.filter((id) => id != log.id);
  } else {
    expandedLogs.value = expandedLogs.value.concat(log.id);
  }
}

// if user is scrolling up manually, disable autoscroll
watchEffect(() => {
  lastScrollY.value = containerScroll.y.value;
});
function disableAutoscrollIfFromUser() {
  // disable it only if the user actually scrolled up
  if (autoscroll.value && containerRef.value != null && containerRef.value.scrollTop < lastScrollY.value) {
    autoscroll.value = false;
  }
}

function scrollToBottom() {
  containerRef.value?.scrollTo({ top: containerRef.value.scrollHeight, behavior: "instant" });
}

// autoscroll to bottom when logs change (if autoscroll is enabled)
watch(
  () => [logsSorted.value, autoscroll.value],
  () => {
    if (autoscroll.value) {
      scrollToBottom();
    }
  }
);

defineExpose({
  logs,
  loading,
  addLogs,
});
</script>
<template>
  <div ref="containerRef" class="relative" @scroll="disableAutoscrollIfFromUser">
    <!-- Actual logs -->
    <!-- (pretty crude for now, missing pagination, detail views, highlights, ...) -->
    <div ref="logsRef" class="flex w-full flex-col" v-if="!loading">
      <!-- Log entry -->
      <span
        v-for="log in logsSorted"
        :key="log.id"
        class="scroll-hidden -mx-0.5 w-full select-text overflow-y-scroll rounded-sm p-0.5 font-mono focus:outline-none"
        :class="[
          highlight && isHighlighted(log) ? 'bg-yellow-100' : '',
          lowlight && !isHighlighted(log) ? 'opacity-50' : '',
          alwaysExpand || expandedLogs.includes(log.id) ? 'whitespace-pre-wrap' : 'max-w-full truncate',
          expandedLogs.includes(log.id) ? 'bg-orange-100 ring-1 ring-orange-600/20 ' : 'hover:bg-orange-50',
        ]"
        @click="() => alwaysExpand || toggleExpanded(log)"
      >
        <span v-if="!hideMetadata" class="mr-2 select-none text-gray-400">
          {{ DateTime.fromISO(log.createdAt).toFormat("HH:mm:ss.SSS") }}
        </span>
        <span :class="log.stream == 'stderr' ? 'text-red-600' : 'text-gray-900'">{{ log.message }}</span>
      </span>
    </div>
    <!-- Loading -->
    <div v-if="loading && !hideIfEmpty" class="h-4 w-full animate-pulse rounded-sm bg-gray-200 opacity-80" />
    <!-- Empty indicator -->
    <div v-else-if="logsSorted.length == 0 && !hideIfEmpty" class="w-full text-center text-gray-400">No logs</div>
    <!-- Autoscroll toggle/indicator on bottom right -->
    <button
      v-if="!loading"
      v-show="containerHeight && logsBounding.height.value > containerHeight"
      class="fixed z-20 rounded-xl bg-white p-1 text-gray-400 shadow-md ring-1 ring-gray-300 transition-colors duration-150 hover:bg-orange-100 hover:text-gray-700"
      :class="[autoscroll ? 'opacity-100' : 'opacity-40 hover:opacity-80']"
      :style="{
        top: `${containerBounding.bottom.value - 30}px`,
        left: `${containerBounding.right.value - 30}px`,
      }"
      @click="() => ((autoscroll = !autoscroll), !autoscroll || scrollToBottom())"
    >
      <ChevronDoubleDownIcon class="h-4 w-4" />
    </button>
  </div>
</template>

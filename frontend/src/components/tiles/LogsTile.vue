<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import { useLogs } from "@/state/session";
import { DateTime } from "luxon";
import { computed, toRef } from "vue";

const props = defineProps<{
  projectId: string;
  projectVersionId: string;
  runnableIds?: string[];
  runId?: string;
  sessionId?: string;
  live?: boolean;
  limit?: number;
}>();

const { logs, loading, addLogs } = useLogs(
  {
    projectId: toRef(props, "projectId"),
    projectVersionId: toRef(props, "projectVersionId"),
    runnableIds: toRef(props, "runnableIds"),
    sessionId: toRef(props, "sessionId"),
    runId: toRef(props, "runId"),
  },
  {
    live: props.live,
    limit: props.limit,
  }
);
const logsSorted = computed(() => logs.value?.slice().sort((a, b) => a.createdAt.localeCompare(b.createdAt)) ?? []);

// appearance
const showTimestamp = true;
const wrap = true;

defineExpose({
  logs,
  loading,
  addLogs,
});
</script>
<template>
  <div class="relative flex flex-col">
    <!-- Actual logs -->
    <span
      v-for="log in logsSorted"
      :key="log.id"
      class="w-full select-text font-mono"
      :class="[wrap ? 'whitespace-normal' : 'whitespace-nowrap']"
    >
      <span v-if="showTimestamp" class="mr-2 select-all text-gray-400">
        {{ DateTime.fromISO(log.createdAt).toFormat("HH:mm:ss.SSS") }}
      </span>
      <span :class="log.stream == 'stderr' ? 'text-red-600' : 'text-gray-900'">{{ log.message }}</span>
    </span>
    <!-- Loading -->
    <BusySpinnerIcon v-if="loading" class="h-4 w-4 animate-spin text-gray-400" />
  </div>
</template>

<script lang="ts" setup>
import { useLogs } from "@/state/session";
import { DateTime } from "luxon";
import { computed, toRef } from "vue";

const props = defineProps<{
  projectId: string;
  projectVersionId?: string;
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
  <div class="flex flex-col font-mono">
    <span v-for="log in logsSorted" :key="log.id" :class="[wrap ? 'whitespace-normal' : 'whitespace-nowrap']">
      <span v-if="showTimestamp" class="mr-2 text-gray-400">
        {{ DateTime.fromISO(log.createdAt).toFormat("HH:mm:ss.SSS") }}
      </span>
      <span :class="log.stream == 'stderr' ? 'text-red-600' : 'text-gray-900'">{{ log.message }}</span>
    </span>
  </div>
</template>

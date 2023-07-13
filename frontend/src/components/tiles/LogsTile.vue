<script lang="ts" setup>
import { useLogs } from "@/state/session";
import { toRef } from "vue";

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

defineExpose({
  logs,
  loading,
  addLogs,
});
</script>
<template>
  <div class="flex flex-col font-mono">
    looogs
    <!-- nocheckin -->
    <div v-for="log in logs" :key="log.id" :class="log.stream == 'stderr' ? 'text-red-500' : 'text-gray-900'">
      {{ log.message }}
    </div>
  </div>
</template>

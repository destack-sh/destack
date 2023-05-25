<script lang="ts" setup>
import { useElementRefs } from "@/composables/useGrid";
import { formatDurationSeconds, useTimeFromNow } from "@/composables/useNow";
import { ExecutionStatus, ExecutionTriggerType, SymbolType } from "@/gql/graphql";
import { useExecutions } from "@/state/executions";
import { computed, ref, toRef } from "vue";

const props = defineProps<{
  symbolId: string;
  symbolType: SymbolType.Task | SymbolType.Code;
  projectId: string;
  projectVersionId?: string;
  includeAncestorVersions?: boolean;
  rootOnly?: boolean;
  live?: boolean;
  limit?: number;
}>();

const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
}>();

const now = useTimeFromNow(100);
const { executions, totalCount } = useExecutions(
  {
    projectId: toRef(props, "projectId"),
    projectVersionId: toRef(props, "projectVersionId"),
    includeAncestorVersions: toRef(props, "includeAncestorVersions"),
    buildIds: ref(null),
    taskIds: computed(() => (props.symbolType === SymbolType.Task ? [props.symbolId] : null)),
    codeIds: computed(() => (props.symbolType === SymbolType.Code ? [props.symbolId] : null)),
  },
  { root: props.rootOnly, live: props.live, first: props.limit ?? 10 }
);
const executionRefs = useElementRefs<HTMLDivElement>();

const executionHeight = 56;

function getStatusColor(status: ExecutionStatus) {
  if (status == ExecutionStatus.Queued || status == ExecutionStatus.Running || status == ExecutionStatus.Scheduled) {
    return "text-gray-700";
  } else if (status == ExecutionStatus.Aborting || status == ExecutionStatus.Aborted) {
    return "text-gray-700";
  } else if (status == ExecutionStatus.Failed) {
    return "text-red-600";
  } else if (status == ExecutionStatus.Completed) {
    return "text-green-700";
  } else {
    return "text-gray-700";
  }
}

function getTriggerLabel(trigger: ExecutionTriggerType) {
  return (
    {
      [ExecutionTriggerType.Job]: "from job",
      [ExecutionTriggerType.UiInteractive]: "by user",
      [ExecutionTriggerType.RestApi]: "via API",
      [ExecutionTriggerType.Manual]: "manually",
    }[trigger] ?? "by a ghost"
  );
}
</script>
<template>
  <div class="">
    <TransitionGroup
      name="fade"
      tag="ul"
      class="relative flex flex-col gap-1 divide-y divide-orange-900 divide-opacity-[12%]"
    >
      <div
        :ref="(el: any) => executionRefs.registerRef(execution.id, el)"
        tabindex="-1"
        v-for="execution in executions"
        :key="execution.id"
        class="rounded-sm] group flex flex-row justify-between p-2 hover:cursor-pointer hover:bg-orange-100"
        :style="{
          height: executionHeight + 'px',
        }"
      >
        <!-- Metadata -->
        <div class="flex flex-col">
          <!-- Status -->
          <span class="transtion-color flex flex-row text-sm font-extrabold" :class="getStatusColor(execution.status)">
            <!-- <svg class="mr-1.5 mt-2 h-[5px] w-[5px]" fill="currentColor" viewBox="0 0 2 2">
              <circle cx="1" cy="1" r="1" />
            </svg> -->
            <FadeTransition mode="out-in">
              <span :key="execution.status">{{ execution.status }}</span>
            </FadeTransition>
            <span class="ml-1.5">{{
              execution.duration != null
                ? formatDurationSeconds(execution.duration)
                : now.getTimeFromNowString(execution.startedAt)
            }}</span>
          </span>
          <!-- Timing & trigger -->
          <span class="flex flex-row gap-1 text-gray-400">
            <!-- Date -->
            <span>{{ now.getTimeFromNowString(execution.startedAt) }}</span>
            <!-- Trigger -->
            <span>{{ getTriggerLabel(execution.triggerType) }}</span>
          </span>
        </div>
      </div>
    </TransitionGroup>
  </div>
</template>
<style scoped>
/* see https://vuejs.org/examples/#list-transition */
/* 1. declare transition */
.fade-move,
.fade-enter-active,
.fade-leave-active {
  transition: all 0.2s cubic-bezier(0.55, 0, 0.1, 1);
}

/* 2. declare enter from and leave to state */
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: scaleY(0.01) translate(30px, 0);
}

/* 3. ensure leaving items are taken out of layout flow so that moving
      animations can be calculated correctly. */
.fade-leave-active {
  position: absolute;
}
</style>

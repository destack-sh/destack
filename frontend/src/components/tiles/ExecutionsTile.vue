<script lang="ts" setup>
import { useElementRefs } from "@/composables/useGrid";
import { formatDurationSeconds, useTimeFromNow } from "@/composables/useNow";
import { ExecutionStatus, ExecutionTriggerType, SymbolType } from "@/gql/graphql";
import { useExecutions } from "@/state/executions";
import {
  ArrowPathIcon,
  CheckCircleIcon,
  ChevronDoubleDownIcon,
  ChevronDoubleUpIcon,
  QuestionMarkCircleIcon,
  XCircleIcon,
} from "@heroicons/vue/24/solid";
import { BoltIcon } from "@heroicons/vue/24/solid";
import { computed, ref, toRef } from "vue";
import StructTile from "@/components/tiles/StructTile.vue";
import ExecutionTraceback from "@/components/basic/ExecutionTraceback.vue";
import { symbolOf, TypeFlag } from "@/state/runtime";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import ValueInterface from "@/components/interfaces/ValueInterface.vue";

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

// state

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
const symbol = computed(() => symbolOf(props.symbolId));
const inputFields = computed(() => symbol.value?.typeNodes?.filter((t) => !(t.flags & TypeFlag.IsOutput)) ?? []);
const outputFields = computed(() => symbol.value?.typeNodes?.filter((t) => t.flags & TypeFlag.IsOutput) ?? []);

// navigation

// display

const previewFields = computed(() => [...inputFields.value.slice(0, 1), ...outputFields.value.slice(0, 2)]);
const executionHeight = 64;
const expandedExecutionHeight = 500;
const metaWidth = 140;
const expandedExecutionId = ref<string | null>(null);

function toggleExpanded(executionId: string) {
  if (expandedExecutionId.value == executionId) {
    expandedExecutionId.value = null;
  } else {
    expandedExecutionId.value = executionId;
  }
}

function isExpanded(executionId: string) {
  return expandedExecutionId.value == executionId;
}

function isMostlyCached(execution: { duration?: number; cachedDuration?: number }) {
  return (
    execution.duration != null &&
    execution.cachedDuration != null &&
    execution.cachedDuration > execution.duration * 0.8
  );
}

function getStatusIcon(status: ExecutionStatus) {
  if (status == ExecutionStatus.Queued || status == ExecutionStatus.Running || status == ExecutionStatus.Scheduled) {
    return ArrowPathIcon;
  } else if (status == ExecutionStatus.Aborting || status == ExecutionStatus.Aborted) {
    return XCircleIcon;
  } else if (status == ExecutionStatus.Failed) {
    return XCircleIcon;
  } else if (status == ExecutionStatus.Completed) {
    return CheckCircleIcon;
  } else {
    return QuestionMarkCircleIcon;
  }
}

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

function getTriggerLabel(execution: { triggerType: ExecutionTriggerType; user?: { slug?: string } }): string {
  return (
    {
      [ExecutionTriggerType.Job]: "from job",
      [ExecutionTriggerType.UiInteractive]: "by " + execution.user?.slug,
      [ExecutionTriggerType.RestApi]: "via API",
      [ExecutionTriggerType.Manual]: "manually",
    }[execution.triggerType] ?? "by a ghost"
  );
}
</script>
<template>
  <div class="flex flex-col">
    <!-- Header with filters  -->

    <!-- Executions -->
    <TransitionGroup name="fade" tag="div" class="relative flex flex-col">
      <div
        :ref="(el: any) => executionRefs.registerRef(execution.id, el)"
        tabindex="-1"
        v-for="(execution, y) in executions"
        :key="execution.id"
        class="group/execution flex flex-col px-1 pt-2 transition"
        :class="[
          y > 0 ? 'border-t border-orange-900 border-opacity-[12%]' : '',
          isExpanded(execution.id) ? '' : 'hover:cursor-pointer hover:bg-orange-100',
        ]"
        :style="{
          height: isExpanded(execution.id) ? undefined : executionHeight + 'px',
        }"
        @click.stop="toggleExpanded(execution.id)"
        @keydown.enter.stop="toggleExpanded(execution.id)"
      >
        <!-- Header -->
        <div class="flex flex-row items-baseline justify-between gap-5 rounded-sm">
          <!-- Metadata -->
          <div class="flex flex-col" :style="{ width: metaWidth + 'px' }">
            <!-- Status & timing -->
            <span
              class="transtion flex flex-row items-center gap-1.5 text-sm font-extrabold"
              :class="getStatusColor(execution.status)"
            >
              <!-- Status -->
              <!-- <svg class="mr-1.5 mt-2 h-[5px] w-[5px]" fill="currentColor" viewBox="0 0 2 2">
              <circle cx="1" cy="1" r="1" />
            </svg> -->
              <FadeTransition mode="out-in">
                <component :is="getStatusIcon(execution.status)" class="h-4 w-4" />
              </FadeTransition>
              <!-- Duration -->
              <span class="">
                {{
                  execution.duration != null
                    ? formatDurationSeconds(execution.duration * 1000)
                    : now.getTimeFromNowString(execution.startedAt)
                }}</span
              >
              <!-- From -->
              <span class="text-gray-400">{{ now.getTimeFromNowString(execution.startedAt) }}</span>
              <!-- Cached info -->
              <span v-if="isMostlyCached(execution as any)" class="group/cache relative">
                <BoltIcon class="h-4 w-4 text-orange-500" />
                <span
                  v-if="execution.duration != null && execution.cachedDuration != null"
                  class="invisible absolute z-10 -ml-1 mt-1 w-44 rounded-sm border border-orange-900 border-opacity-[12%] bg-white px-2 py-1 text-gray-700 group-hover/cache:visible"
                >
                  Cached
                  {{ now.getTimeFromNowString(execution.cachedGeneratedAt) }} ago<br />
                  Saved {{ (100 - (execution.duration * 100) / execution.cachedDuration).toFixed() }}% (~{{
                    formatDurationSeconds((execution.cachedDuration - execution.duration) * 1000)
                  }})
                </span>
              </span>
            </span>
            <!-- Trigger -->
            <span class="flex w-full flex-row gap-1 text-gray-400">
              <span class="truncate">{{ getTriggerLabel(execution) }}</span>
            </span>
          </div>
          <!-- Selected fields -->
          <ValueInterface
            v-for="field in previewFields"
            :key="field"
            :type="field"
            readonly
            active
            :model-value="execution.inputs?.[field.key] ?? execution.outputs?.[field.key]"
            class="w-4 overflow-hidden"
            :style="{
              height: executionHeight + 'px',
            }"
          />
          <!-- Controls -->
          <button class="p-0.5 text-gray-400 hover:bg-orange-100 hover:text-gray-700">
            <component :is="isExpanded(execution.id) ? ChevronDoubleUpIcon : ChevronDoubleDownIcon" class="h-4 w-4" />
          </button>
        </div>
        <!-- Body / execution tile preview -->
        <div
          v-if="expandedExecutionId === execution.id"
          class="scroll-hidden my-3 w-full gap-5 overflow-y-auto"
          :style="{
            maxHeight: expandedExecutionHeight + 'px',
          }"
        >
          <StructTile
            v-if="execution.inputs != null"
            readonly
            class="w-full"
            :fields="[...inputFields, ...(execution.outputs != null ? outputFields : [])]"
            :model-value="{ ...(execution.inputs ?? {}), ...(execution.outputs ?? {}) }"
          />
          <ExecutionTraceback v-if="execution.errorNice" class="w-full p-1" name="run" :execution="execution" />
        </div>
      </div>
    </TransitionGroup>
    <!-- Load more -->
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

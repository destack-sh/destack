<script lang="ts" setup>
import ErrorTraceback from "@/components/basic/ErrorTraceback.vue";
import ValueInterface from "@/components/interfaces/ValueInterface.vue";
import StructTile from "@/components/tiles/StructTile.vue";
import { useElementRefs } from "@/composables/useGrid";
import { formatDurationSeconds, useTimeFromNow } from "@/composables/useNow";
import { RunStatus, RunTriggerType } from "@/gql/graphql";
import { useSessions, isMostlyCached, getCachedPercentage } from "@/state/session";
import { useCurrentModule, TypeFlag } from "@/state/module";
import {
  BoltIcon,
  CheckCircleIcon,
  ChevronDoubleDownIcon,
  ChevronDoubleUpIcon,
  QuestionMarkCircleIcon,
  XCircleIcon,
} from "@heroicons/vue/24/solid";
import { useElementSize } from "@vueuse/core";
import { computed, ref, toRef, type Ref } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";

const props = defineProps<{
  runnableId: string;
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

const module = useCurrentModule();
const now = useTimeFromNow(100);
const { runs, totalCount, loading } = useSessions(
  {
    projectId: toRef(props, "projectId"),
    projectVersionId: toRef(props, "projectVersionId"),
    includeAncestorVersions: toRef(props, "includeAncestorVersions"),
    runnableIds: computed(() => [props.runnableId]),
  },
  { root: props.rootOnly, live: props.live, first: props.limit ?? 10 }
);
const runRefs = useElementRefs<HTMLDivElement>();
const statement = computed(() => module.statementOf(props.runnableId));
const inputFields = computed(() => statement.value?.fields?.filter((t) => !(t.flags & TypeFlag.IsOutput)) ?? []);
const outputFields = computed(() => statement.value?.fields?.filter((t) => t.flags & TypeFlag.IsOutput) ?? []);
const expandedRunId = ref<string | null>(null);

// navigation

// display

const containerRef: Ref<HTMLDivElement | null> = ref(null);
const containerSize = useElementSize(containerRef);
const previewFields = computed(() => [...inputFields.value.slice(0, 1), ...outputFields.value.slice(0, 2)]);
const headerHeight = 64;
const bodyHeight = 512;
const paddingY = 8;
const paddingX = 4;
const metadataWidth = 200;
const previewWidth = computed(() => {
  return containerSize.width.value - metadataWidth - paddingX * 2;
});

function toggleExpanded(runId: string) {
  if (expandedRunId.value == runId) {
    expandedRunId.value = null;
  } else {
    expandedRunId.value = runId;
  }
}

function isExpanded(runId: string) {
  return expandedRunId.value == runId;
}

function getStatusIcon(status: RunStatus) {
  if (status == RunStatus.Queued || status == RunStatus.Running) {
    return BusySpinnerIcon;
  } else if (status == RunStatus.Aborting || status == RunStatus.Aborted) {
    return XCircleIcon;
  } else if (status == RunStatus.Failed) {
    return XCircleIcon;
  } else if (status == RunStatus.Completed) {
    return CheckCircleIcon;
  } else {
    return QuestionMarkCircleIcon;
  }
}

function getStatusColor(status: RunStatus) {
  if (status == RunStatus.Queued || status == RunStatus.Running || status == RunStatus.Scheduled) {
    return "text-gray-700";
  } else if (status == RunStatus.Aborting || status == RunStatus.Aborted) {
    return "text-gray-700";
  } else if (status == RunStatus.Failed) {
    return "text-red-600";
  } else if (status == RunStatus.Completed) {
    return "text-green-700";
  } else {
    return "text-gray-700";
  }
}

function getTriggerLabel(run: { triggerType: RunTriggerType; user?: { slug?: string | null } }): string {
  return (
    {
      [RunTriggerType.Ui]: "by " + run.user?.slug,
      [RunTriggerType.Api]: "via API",
      [RunTriggerType.Reactive]: "reactively",
      [RunTriggerType.Scheduled]: "scheduled",
    }[run.triggerType] ?? "by a ghost"
  );
}
</script>
<template>
  <div ref="containerRef" class="flex flex-col">
    <!-- Header with filters  -->
    <!-- not yet -->
    <!-- Runs -->
    <div v-if="totalCount == 0" class="flex h-full w-full items-center justify-center text-gray-400">No runs</div>
    <div v-else-if="loading" class="flex h-full w-full items-center justify-center">
      <BusySpinnerIcon class="h-4 w-4 animate-spin text-gray-500" />
    </div>
    <!-- TODO @UX: animate runs in tile (without interfering with expand/close animation, looks glitchy) -->
    <div v-else class="relative flex flex-col">
      <div
        :ref="(el: any) => runRefs.registerRef(run.id, el)"
        tabindex="-1"
        v-for="(run, y) in runs"
        :key="run.id"
        class="group/run flex flex-col"
        :class="[y > 0 ? 'border-t- border-orange-900 border-opacity-[12%]' : '']"
        :style="{
          paddingTop: paddingY + 'px',
          paddingBottom: paddingY + 'px',
          paddingLeft: paddingX + 'px',
          paddingRight: paddingX + 'px',
          height: isExpanded(run.id) ? undefined : headerHeight + 'px',
        }"
        @click.stop="toggleExpanded(run.id)"
        @keydown.enter.stop="toggleExpanded(run.id)"
      >
        <!-- Header -->
        <div class="flex flex-row justify-between gap-5 rounded-sm">
          <!-- Metadata -->
          <div class="flex flex-col self-start" :style="{ width: metadataWidth + 'px' }">
            <!-- Status & timing -->
            <span class="transtion flex max-w-full flex-row items-center" :class="getStatusColor(run.status)">
              <!-- Status -->
              <component
                :is="getStatusIcon(run.status)"
                class="h-4 w-4"
                :class="[
                  run.status == RunStatus.Running || run.status == RunStatus.Queued
                    ? 'animate-spin'
                    : '',
                ]"
              />
              <span class="ml-1 max-w-full truncate font-semibold">{{ statement?.name }}</span>
              <!-- Duration -->
              <span class="group/cache ml-1 flex flex-row">
                {{
                  run.duration != null
                    ? formatDurationSeconds(run.duration * 1000)
                    : now.getTimeFromNowString(run.startedAt)
                }}
                <!-- Cached info :CacheInfo -->
                <span v-if="isMostlyCached(run as any)" class="relative px-0.5 py-1">
                  <BoltIcon class="h-3 w-3 text-orange-500" />
                  <span
                    v-if="run.duration != null && run.cachedDuration != null"
                    class="invisible absolute z-10 -ml-1 mt-1 w-36 rounded-sm border border-orange-900 border-opacity-[12%] bg-white px-2 py-1 text-xs text-gray-700 group-hover/cache:visible"
                  >
                    Cached
                    {{ now.getTimeFromNowString(run.cachedGeneratedAt) }} ago<br />
                    <template v-if="getCachedPercentage(run) > 0">
                      Saved {{ getCachedPercentage(run).toFixed() }}% (~{{
                        formatDurationSeconds((run.cachedDuration - run.duration) * 1000)
                      }})
                    </template>
                  </span>
                </span>
              </span>
            </span>
            <!-- Trigger -->
            <span class="flex w-full flex-row gap-1 text-gray-400">
              <!-- From -->
              <span class="text-gray-400">{{ now.getTimeFromNowString(run.startedAt) }}</span>
              <span class="truncate">{{ getTriggerLabel(run) }}</span>
            </span>
          </div>
          <!-- Selected fields as a preview -->
          <!-- TODO @UX: select and render run fields preview more intelligently -->
          <div
            class="relative flex w-full flex-row justify-normal gap-x-3 overflow-hidden"
            :style="{
              width: previewWidth + 'px',
              maxWidth: previewWidth + 'px',
            }"
          >
            <ValueInterface
              v-for="field in previewFields"
              :key="field.id"
              :type="module.effectiveTypeOf(field)"
              readonly
              active
              wrap
              :model-value="run.inputs?.[module.getTypedKey(field) as string] ?? run.outputs?.[module.getTypedKey(field) as string]"
              class="overflow-hidden"
              :style="{
                // 12 = gap-x-3
                width: previewWidth / previewFields.length - (12 * previewFields.length - 1) + 'px',
                height: headerHeight - 24 + 'px',
              }"
            />
            <!-- fade to white towards bottom -->
            <div
              class="pointer-events-none absolute bottom-0 left-0 h-5 w-full bg-gradient-to-tl from-white to-transparent"
            >
              &nbsp;
            </div>
          </div>
          <!-- Controls -->
          <button class="self-start p-0.5 text-gray-400 hover:bg-orange-100 hover:text-gray-700">
            <component :is="isExpanded(run.id) ? ChevronDoubleUpIcon : ChevronDoubleDownIcon" class="h-4 w-4" />
          </button>
        </div>
        <!-- Body / run tile preview -->
        <div
          v-if="expandedRunId === run.id"
          @click.stop
          class="scroll-hidden my-3 w-full gap-5 overflow-y-auto"
          :style="{
            maxHeight: bodyHeight + 'px',
          }"
        >
          <StructTile
            v-if="run.inputs != null"
            readonly
            class="w-full"
            :fields="[...inputFields, ...(run.outputs != null ? outputFields : [])]"
            :model-value="{ ...(run.inputs ?? {}), ...(run.outputs ?? {}) }"
          />
          <ErrorTraceback
            v-if="run.errorNice"
            class="w-full rounded-sm border border-orange-900 border-opacity-[12%] p-1 font-mono"
            name="run"
            :run="run"
          />
        </div>
      </div>
    </div>
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

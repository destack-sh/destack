<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import PanelHeader from "@/components/editors/PanelHeader.vue";
import ContainerTile from "@/components/tiles/ContainerTile.vue";
import RunsTile from "@/components/tiles/RunsTile.vue";
import StructTile from "@/components/tiles/StructTile.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { RunStatus, StatementType } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type PanelContext, type StatementAction, type QuickRunPanel } from "@/state/bench";
import { newRunId, newSessionId, TypeFlag, useCurrentModule } from "@/state/module";
import { PlayIcon } from "@heroicons/vue/24/solid";
import { computed, ref, watch, watchEffect } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import TraceTile from "@/components/tiles/TraceTile.vue";
import { ACTIVE_RUN_STATUSES, TERMINAL_RUN_STATUSES, useCurrentSessions } from "@/state/session";
import { StopIcon } from "@heroicons/vue/24/outline";

const INLINE_RUNS_LIMIT = 10;
const props = defineProps<{ panel: PanelContext<QuickRunPanel>; focused: boolean }>();
const emit = defineEmits<{
  (e: "close"): void;
}>();

const bench = useBenchState();
const appearance = useAppearance();
const panel = computed(() => props.panel.panel.value);
const editorSize = computed(() => props.panel.size.value);
const now = useTimeFromNow();

// state

const module = useCurrentModule();
const sessions = useCurrentSessions();
const statement = computed(() => module.statementOf(props.panel.panel.value.statementId));
const inputFields = computed(() => statement.value?.fields?.filter((t) => !(t.flags & TypeFlag.IsOutput)) ?? []);
const outputFields = computed(() => statement.value?.fields?.filter((t) => t.flags & TypeFlag.IsOutput) ?? []);
const terminalActions = computed(() => {
  const actions: StatementAction[] = [
    {
      label: "Run",
      icon: PlayIcon,
      action: () => run(),
      active: isCurrentRunActive.value,
      disabled: statement.value == null,
      hideInline: true,
    },
    {
      label: "Cancel",
      icon: StopIcon,
      action: () => cancel(),
      disabled: !isCurrentRunActive.value,
      hideInline: true,
    },
  ];
  return actions;
});

// sync symbol type into editor
watchEffect(() => {
  if (statement.value?.type != null && statement.value.type != panel.value.statementType) {
    if (![StatementType.Code, StatementType.Task].includes(statement.value.type)) {
      throw new Error(`unexpected statement type ${statement.value.type}`);
    }
    panel.value.statementType = statement.value.type as StatementType.Code | StatementType.Task;
  }
});

// sync name/path into editor
const path = computed(() => {
  if (statement.value == null) return null;
  if (module.fileOf(statement.value) == null) return null;
  return module.fileOf(statement.value)?.name + ":" + statement.value?.name;
});
watch(path, () => {
  if (statement.value == null || module.idx.value == null) return;
  panel.value.updatePath(statement.value, module.idx.value);
});

const statementPath = computed(() => module.nodePathOf({ id: panel.value.statementId }));

// running

const runs = sessions.runsOf({ id: panel.value.statementId });
const currentRun = computed(() => runs.value[0]);
const isCurrentRunActive = computed(
  () =>
    currentRun.value != null &&
    ACTIVE_RUN_STATUSES.includes(currentRun.value?.status) &&
    currentRun.value?.status != RunStatus.Aborting
);

async function run() {
  if (statement.value == null) return;
  panel.value.lastRunId = newRunId();
  panel.value.lastSessionId = newSessionId();
  panel.value.lastOutput = undefined;
  const { run, result: runTask } = await sessions.run(
    { id: panel.value.statementId },
    {
      inputs: panel.value.inputs,
      keyed: true,
      runId: panel.value.lastRunId,
      sessionId: panel.value.lastSessionId,
    }
  );
  const result = await runTask;

  if (TERMINAL_RUN_STATUSES.includes(result?.run.status)) {
    panel.value.lastRunTerminatedAt = result?.run.terminatedAt;
    panel.value.lastOutput = result?.run.outputs;
  } else {
    // subscribe to run changes
    sessions.subscribeToRun(run, (run) => {
      if (TERMINAL_RUN_STATUSES.includes(run.status)) {
        panel.value.lastRunTerminatedAt = run.terminatedAt;
        panel.value.lastOutput = run.outputs;
      }
    });
  }
}

async function cancel() {
  if (!isCurrentRunActive.value) return;
  await sessions.cancel(currentRun.value);
}

// tiling
const gridStepX = ref(36); // p-9
const gridStepY = ref(18); // p-4.5

function getTileWidth(targetWidth?: number) {
  return Math.min(
    targetWidth ?? panel.value.contentWidth,
    props.panel.size.value.width - 2 * panel.value.contentMarginX
  );
}

function getTileOffsetX(targetWidth?: number) {
  return (props.panel.size.value.width - getTileWidth(targetWidth)) / 2;
}

function getTilePositionX(targetWidth?: number) {
  const tileWidth = getTileWidth(targetWidth);
  const tileOffsetX = getTileOffsetX(targetWidth);
  return {
    width: tileWidth + "px",
    marginLeft: tileOffsetX + "px",
    marginRight: tileOffsetX + "px",
  };
}

const baseTilePositionX = computed(() => getTilePositionX());

// navigation

function focus() {
  // nothing to do yet
}

function blur() {
  // nothing to do yet
}

defineExpose({
  focus,
  blur,
});
</script>
<template>
  <div class="relative flex flex-col" :style="{ minHeight: editorSize.height + 'px' }">
    <!-- Fixed inline header -->
    <PanelHeader
      class="border-b border-orange-900 border-opacity-[12%]"
      :editing="false"
      :thing="statement"
      :actions="terminalActions"
      :path="statementPath ?? []"
      :self="(statementPath?.length ?? 0) - 1"
    />
    <!-- Tiles -->
    <div
      class="relative flex h-full w-full flex-col gap-6"
      :class="appearance.baseClass"
      :style="{
        marginTop: appearance.editorHeaderHeight + 'px',
        paddingTop: gridStepY + 'px',
        paddingBottom: gridStepY + 'px',
        minHeight: editorSize.height - appearance.editorHeaderHeight + 'px',
      }"
    >
      <!-- Header -->
      <div class="z-[1] flex flex-row items-baseline justify-between p-2" :style="baseTilePositionX">
        <!-- Title & source -->
        <div class="flex flex-col">
          <h1 class="text-3xl font-bold text-gray-900">{{ statement?.name ?? "" }}&nbsp;</h1>
        </div>
        <!-- Run button -->
        <div
          class=""
          :style="{
            height: gridStepY * 2 + 'px',
            width: getTileWidth(gridStepX * 3) + 'px',
          }"
        >
          <button
            class="flex h-full w-full flex-row items-center justify-center gap-1 rounded-sm bg-orange-500 text-white hover:bg-orange-400 focus:bg-orange-400"
            @click="isCurrentRunActive ? cancel() : run()"
            @keydown.enter.exact.prevent="run"
          >
            Run
            <FadeTransition>
              <PlayIcon v-if="!isCurrentRunActive" class="h-4 w-4" />
            </FadeTransition>
          </button>
        </div>
      </div>
      <div v-if="module.loading.value" class="flex w-full flex-1 flex-col items-center justify-center">
        <BusySpinnerIcon class="mx-auto h-8 w-8 animate-spin text-white" />
      </div>
      <template v-else>
        <!-- Input -->
        <ContainerTile label="Input" :style="{ ...baseTilePositionX }">
          <StructTile
            v-if="inputFields.length > 0"
            v-model="panel.inputs"
            :fields="inputFields"
            full-inputs
            readonly-type
            class=""
          />
          <div v-else class="flex h-full w-full flex-col items-center justify-center">
            <span class="text-sm text-gray-400">No input</span>
          </div>
        </ContainerTile>
        <!-- Trace -->
        <ContainerTile
          label="Trace"
          :sub-label="
            panel.lastRunTerminatedAt != null
              ? now.getTimeFromNowLongString(panel.lastRunTerminatedAt as string)
              : undefined
          "
          :style="{ ...baseTilePositionX }"
        >
          <TraceTile v-if="panel.lastRunId" :root-id="panel.lastRunId" layout="list" live />
          <div v-else class="flex h-full w-full flex-col items-center justify-center">
            <span class="text-sm text-gray-400">No trace</span>
          </div>
        </ContainerTile>
        <!-- Output -->
        <ContainerTile
          label="Output"
          :sub-label="
            panel.lastRunTerminatedAt != null
              ? now.getTimeFromNowLongString(panel.lastRunTerminatedAt as string)
              : undefined
          "
          :style="{ ...baseTilePositionX }"
        >
          <StructTile
            v-if="panel.lastOutput && outputFields.length > 0"
            :model-value="panel.lastOutput"
            :fields="outputFields"
            readonly
            class=""
          />
          <div v-else class="flex h-full w-full flex-col items-center justify-center">
            <span class="text-sm text-gray-400">No output</span>
          </div>
        </ContainerTile>
        <!-- Runs -->
        <ContainerTile v-if="statement != null" label="Runs" :style="{ ...baseTilePositionX }">
          <RunsTile
            :project-id="(bench.projectId as string)"
            :project-version-id="(bench.projectVersionId as string)"
            include-ancestor-versions
            :runnable-id="panel?.statementId"
            :symbol-type="statement?.type"
            live
            :limit="INLINE_RUNS_LIMIT"
          />
        </ContainerTile>
      </template>
    </div>
  </div>
</template>

<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import PanelHeader from "@/components/panels/PanelHeader.vue";
import ContainerTile from "@/components/tiles/ContainerTile.vue";
import RunsTile from "@/components/tiles/RunsTile.vue";
import StructTile from "@/components/tiles/StructTile.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { RunStatus, StatementType } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type PanelContext, type StatementAction, type LaunchRunPanel } from "@/state/bench";
import { newRunId, newSessionId, TypeFlag, useCurrentModule } from "@/state/module";
import { PlayIcon } from "@heroicons/vue/24/solid";
import { computed, ref, watch, watchEffect } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import TraceTile from "@/components/tiles/TraceTile.vue";
import { ACTIVE_RUN_STATUSES, TERMINAL_RUN_STATUSES, useCurrentSessions } from "@/state/session";
import { StopIcon } from "@heroicons/vue/24/outline";
import { useTiling } from "@/state/screen";
import ErrorTraceback from "@/components/basic/ErrorTraceback.vue";

const RUNS_HISTORY_LIMIT = 15;
const props = defineProps<{ panel: PanelContext<LaunchRunPanel>; focused: boolean }>();
const emit = defineEmits<{
  (e: "close"): void;
}>();

const bench = useBenchState();
const appearance = useAppearance();
const panel = computed(() => props.panel.panel.value);
const panelSize = computed(() => props.panel.size.value);
const now = useTimeFromNow();

// state

const module = useCurrentModule();
const sessions = useCurrentSessions();
const statement = computed(() => module.statementOf(props.panel.panel.value.statementId));
const inputFields = computed(
  () => statement.value?.fields?.filter((t) => t.deletedAt == null && !(t.flags & TypeFlag.IsOutput)) ?? []
);
const outputFields = computed(
  () => statement.value?.fields?.filter((t) => t.deletedAt == null && t.flags & TypeFlag.IsOutput) ?? []
);
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
  panel.value.lastError = undefined;
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
    panel.value.lastError = result?.run.error;
  } else {
    // subscribe to run changes
    sessions.subscribeToRun(run, (run) => {
      if (TERMINAL_RUN_STATUSES.includes(run.status)) {
        panel.value.lastRunTerminatedAt = run.terminatedAt;
        panel.value.lastOutput = run.outputs;
        panel.value.lastError = run.error;
      }
    });
  }
}

async function cancel() {
  if (!isCurrentRunActive.value) return;
  await sessions.cancel(currentRun.value);
}

const runsTileRef = ref<InstanceType<typeof RunsTile> | null>(null);
const { gridStepX, gridStepY, getTileWidth, baseTilePositionX } = useTiling(props.panel);

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
  <div class="relative flex flex-col" :style="{ minHeight: panelSize.height + 'px' }">
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
        minHeight: panelSize.height - appearance.editorHeaderHeight + 'px',
      }"
    >
      <!-- Header -->
      <div class="z-[1] flex flex-row items-baseline justify-between p-2" :style="baseTilePositionX">
        <!-- Title & source -->
        <h1 class="text-3xl font-bold text-gray-900">Run: {{ statement?.name ?? "" }}&nbsp;</h1>
        <!-- Run controls -->
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
      <!-- Body -->
      <div v-if="module.loading.value" class="flex w-full flex-1 flex-col items-center justify-center">
        <BusySpinnerIcon class="mx-auto h-8 w-8 animate-spin text-white" />
      </div>
      <template v-else>
        <!-- Input -->
        <ContainerTile label="Input" :style="{ ...baseTilePositionX }">
          <span v-if="inputFields?.length == 0" class="w-full text-center text-gray-400">No inputs</span>
          <StructTile v-model="panel.inputs" :fields="inputFields" full-inputs readonly-type class="" />
        </ContainerTile>
        <!-- Trace -->
        <ContainerTile
          v-if="panel.lastRunId"
          label="Trace"
          :sub-label="
            panel.lastRunTerminatedAt != null
              ? now.getTimeFromNowLongString(panel.lastRunTerminatedAt as string)
              : undefined
          "
          :style="{ ...baseTilePositionX }"
        >
          <TraceTile :root-id="panel.lastRunId" layout="list" live />
        </ContainerTile>
        <!-- Output -->
        <ContainerTile
          v-if="panel.lastOutput != null"
          label="Output"
          :sub-label="
            panel.lastRunTerminatedAt != null
              ? now.getTimeFromNowLongString(panel.lastRunTerminatedAt as string)
              : undefined
          "
          :style="{ ...baseTilePositionX }"
        >
          <span v-if="outputFields?.length == 0" class="w-full text-center text-gray-400">No outputs</span>
          <StructTile :model-value="panel.lastOutput" :fields="outputFields" readonly class="" />
        </ContainerTile>
        <!-- Runs  -->
        <ContainerTile
          v-if="statement != null"
          label="Runs"
          :sub-label="
            runsTileRef?.totalCount != null
              ? `last ${Math.min(RUNS_HISTORY_LIMIT, runsTileRef?.totalCount)} of ${runsTileRef?.totalCount}`
              : undefined
          "
          :style="{ ...baseTilePositionX }"
        >
          <template v-slot:sublabel>
            <button
              class="font-normal underline-offset-2 hover:underline"
              @click="bench.openRuns(undefined, { focus: true })"
            >
              (view all)
            </button>
          </template>
          <RunsTile
            ref="runsTileRef"
            :project-id="(bench.projectId as string)"
            :project-version-id="(bench.projectVersionId as string)"
            :runnable-ids="[panel.statementId]"
            live
            :limit="RUNS_HISTORY_LIMIT"
            class="max-w-full overflow-x-auto"
            hide-header
          >
          </RunsTile>
        </ContainerTile>
      </template>
    </div>
  </div>
</template>

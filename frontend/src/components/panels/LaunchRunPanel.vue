<script lang="ts" setup>
import PanelHeader from "@/components/panels/PanelHeader.vue";
import ContainerTile from "@/components/tiles/ContainerTile.vue";
import RunsTile from "@/components/tiles/RunsTile.vue";
import StructInterface from "@/components/interfaces/StructInterface.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { RunStatus, StatementType, type Run } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type PanelContext, type StatementAction, type LaunchRunPanel } from "@/state/bench";
import { TypeFlag, useCurrentModule, type Statement } from "@/state/module";
import { PlayIcon } from "@heroicons/vue/24/solid";
import { computed, ref, watch, watchEffect, type Ref } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import TraceTile from "@/components/tiles/TraceTile.vue";
import { ACTIVE_RUN_STATUSES, TERMINAL_RUN_STATUSES, useCurrentSessions } from "@/state/session";
import { BoltIcon, LightBulbIcon, StopIcon } from "@heroicons/vue/24/outline";
import { useTiling } from "@/state/screen";
import PanelStatusNotice from "@/components/panels/PanelStatusNotice.vue";
import ErrorTraceback from "@/components/basic/ErrorTraceback.vue";
import RunControlsTile from "@/components/tiles/RunControlsTile.vue";
import LogsTile from "@/components/tiles/LogsTile.vue";

const RUNS_HISTORY_LIMIT = 20;
const props = defineProps<{ panel: PanelContext<LaunchRunPanel>; focused: boolean }>();
const emit = defineEmits<{
  (e: "close"): void;
}>();

const bench = useBenchState();
const appearance = useAppearance();
const panel = computed(() => props.panel.panel.value);
const panelSize = computed(() => props.panel.size.value);
const now = useTimeFromNow();
const module = useCurrentModule();
const sessions = useCurrentSessions();
const taskRunModeKey = computed(() => module.taskRunConfigKey("mode"));

const taskRunMode: Ref<"fast" | "deliberate"> = computed({
  get: () => panel.value?.inputs?.[taskRunModeKey.value ?? ""] ?? "fast",
  set: (v) => {
    if (panel.value == null || taskRunModeKey.value == null) return;
    panel.value.inputs[taskRunModeKey.value] = v;
  },
});

// state

const statement = computed(() => module.statementOf(props.panel.panel.value.statementCk));
const inputFields = computed(
  () => statement.value?.fields?.filter((t) => t.deletedAt == null && !(t.flags & TypeFlag.IS_OUTPUT)) ?? []
);
const outputFields = computed(
  () => statement.value?.fields?.filter((t) => t.deletedAt == null && t.flags & TypeFlag.IS_OUTPUT) ?? []
);
const actions = computed(() => {
  const actions: StatementAction[] = [
    {
      label: "Run",
      icon: PlayIcon,
      action: () => run(),
      active: isCurrentRunActive.value,
      disabled: statement.value == null || !bench.canUse,
      hideInline: true,
    },
    {
      label: "Cancel",
      icon: StopIcon,
      action: () => cancel(),
      disabled: !isCurrentRunActive.value || !bench.canUse,
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
  return module.pathOf(panel.value.statementCk);
});
watch(path, () => {
  if (statement.value == null || module.idx.value == null) return;
  panel.value.updatePath(statement.value, module.idx.value);
});

const statementPath = computed(() => module.nodePathOf(panel.value.statementCk));

// running

const runs = sessions.runsOf({ ck: panel.value.statementCk });
const currentRun = computed(() => runs.value[0]);
const isCurrentRunActive = computed(
  () =>
    currentRun.value != null &&
    ACTIVE_RUN_STATUSES.includes(currentRun.value?.status) &&
    currentRun.value?.status != RunStatus.Aborting
);
const subscribedToCurrentRun = ref(false);

// subscribe if we triggered a run that's active but
watch(isCurrentRunActive, (active) => {
  if (active && !subscribedToCurrentRun.value) {
    subscribeUntilTermination(currentRun.value as Run);
  }
});

function subscribeUntilTermination(run: Run) {
  // subscribe to run changes until termination
  sessions.subscribeToRun(run, (run) => {
    if (TERMINAL_RUN_STATUSES.includes(run.status)) {
      panel.value.lastRunTerminatedAt = run.terminatedAt;
      panel.value.lastOutput = run.outputs;
      panel.value.lastError = run.errorNice;
    }
  });
  subscribedToCurrentRun.value = true;
}

async function run() {
  if (statement.value == null) return;
  subscribedToCurrentRun.value = false;
  const { run, firstResult: runTask } = await sessions.run(
    { id: statement.value.id, ck: statement.value.ck },
    {
      inputs: panel.value.inputs,
      keyed: true,
      runId: panel.value.lastRunId,
      sessionId: panel.value.lastSessionId,
    }
  );
  const result = await runTask;
  onRun(result.run ?? run);
}

async function cancel() {
  if (!isCurrentRunActive.value) return;
  await sessions.kill(currentRun.value);
}

async function onRun(run: Run) {
  panel.value.inputs = run.inputs;
  panel.value.lastRunId = run.id;
  panel.value.lastSessionId = run.session?.id;
  panel.value.lastRunTerminatedAt = undefined;
  panel.value.lastOutput = undefined;
  panel.value.lastError = undefined;

  if (TERMINAL_RUN_STATUSES.includes(run.status)) {
    panel.value.lastRunTerminatedAt = run.terminatedAt;
    panel.value.lastOutput = run.outputs;
    panel.value.lastError = run.errorNice;
  } else {
    subscribeUntilTermination(run);
  }
}

const runsTileRef = ref<InstanceType<typeof RunsTile> | null>(null);
const { gridStepY, baseTilePositionX } = useTiling(props.panel);

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
      class="border-b border-orange-900/[12%] bg-gray-50"
      :editing="false"
      :thing="statement"
      :actions="actions"
      :path="statementPath ?? []"
      :self="(statementPath?.length ?? 0) - 1"
    />
    <PanelStatusNotice :thing="statement" name="statement" :loading="module.loading.value" />
    <!-- Tiles -->
    <div
      v-if="statement != null"
      class="relative flex h-full w-full flex-col gap-6"
      :class="appearance.baseClass"
      :style="{
        marginTop: appearance.panelHeaderHeight + 'px',
        paddingTop: gridStepY + 'px',
        paddingBottom: gridStepY + 'px',
        minHeight: panelSize.height - appearance.panelHeaderHeight + 'px',
      }"
    >
      <!-- Header -->
      <div class="z-[1] flex flex-row items-baseline justify-between p-2" :style="baseTilePositionX">
        <!-- Title & source -->
        <h1 class="text-3xl font-bold text-gray-900">Run: {{ statement?.name ?? "(unnamed)" }}&nbsp;</h1>
        <!-- Run controls -->
        <RunControlsTile
          :run="currentRun"
          :statement="statement"
          :inputs="panel.inputs"
          @rerun="onRun"
          @run="onRun"
          :hide="['rerun', 'launch']"
        />
      </div>
      <!-- Body -->
      <div v-if="module.loading.value" class="flex w-full flex-1 flex-col items-center justify-center">
        <BusySpinnerIcon class="mx-auto h-8 w-8 animate-spin text-white" />
      </div>
      <template v-else>
        <!-- Input -->
        <ContainerTile label="Input" :style="{ ...baseTilePositionX }">
          <span v-if="inputFields?.length == 0" class="w-full text-center text-gray-400">No inputs</span>
          <StructInterface
            v-model="panel.inputs"
            :type="(statement as Statement)"
            :is-output="false"
            full-inputs
            readonly-type
            :appearance="{ minimalFields: true, hideFieldType: true }"
          />
          <!-- TODO @UX: where to put task config in launch run panel? (panel needs a general cleanup anyway) -->
          <button
            v-if="statement.type == StatementType.Task"
            class="ml-auto flex flex-row items-center gap-0.5 rounded-sm text-xs text-gray-400 transition-opacity duration-500 hover:bg-orange-100 hover:text-gray-700"
            @click="taskRunMode = taskRunMode == 'fast' ? 'deliberate' : 'fast'"
          >
            <component :is="taskRunMode == 'fast' ? BoltIcon : LightBulbIcon" class="h-4 w-4" />
            {{ taskRunMode == "fast" ? "Fast" : "Deliberate" }}
          </button>
        </ContainerTile>
        <!-- Output -->
        <ContainerTile
          v-if="panel.lastOutput != null"
          label="Output"
          :sublabel="
            panel.lastRunTerminatedAt != null
              ? now.getTimeFromNowLongString(panel.lastRunTerminatedAt as string)
              : undefined
          "
          sublabel-position="opposite"
          :style="{ ...baseTilePositionX }"
        >
          <span v-if="outputFields?.length == 0" class="w-full text-center text-gray-400">No outputs</span>
          <StructInterface
            :model-value="panel.lastOutput"
            :type="(statement as Statement)"
            :is-output="true"
            show-controls
            readonly
            :appearance="{ minimalFields: true, hideFieldType: true, view: 'tree' }"
          />
        </ContainerTile>
        <!-- Error -->
        <ContainerTile
          v-if="panel.lastError != null"
          label="Error"
          :sublabel="
            panel.lastRunTerminatedAt != null
              ? now.getTimeFromNowLongString(panel.lastRunTerminatedAt as string)
              : undefined
          "
          sublabel-position="opposite"
          :style="{ ...baseTilePositionX }"
        >
          <ErrorTraceback :statement-ck="panel.statementCk" :error-nice="panel.lastError" class="p-1" />
        </ContainerTile>
        <!-- Trace -->
        <ContainerTile v-if="panel.lastRunId" label="Trace" :style="{ ...baseTilePositionX }">
          <TraceTile :root-id="panel.lastRunId" layout="list" live />
        </ContainerTile>
        <!-- Logs -->
        <ContainerTile v-if="panel.lastSessionId" label="Logs" :style="{ ...baseTilePositionX }">
          <LogsTile
            :project-id="(bench.projectId as string)"
            :project-version-id="(bench.projectVersionId as string)"
            :session-id="panel.lastSessionId"
            live
            class="px-1"
          />
        </ContainerTile>
        <!-- Runs  -->
        <ContainerTile
          v-if="statement != null"
          label="Runs"
          :sublabel="
            runsTileRef?.totalCount != null
              ? `last ${Math.min(RUNS_HISTORY_LIMIT, runsTileRef?.totalCount)} of ${runsTileRef?.totalCount}`
              : undefined
          "
          sublabel-position="opposite"
          :style="{ ...baseTilePositionX }"
          class="overflow-x-hidden"
        >
          <template v-slot:sublabel>
            <button
              class="font-normal underline-offset-2 hover:underline"
              @click="bench.openViewRuns(undefined, { focus: true })"
            >
              (view all)
            </button>
          </template>
          <RunsTile
            ref="runsTileRef"
            :project-id="(bench.projectId as string)"
            :project-version-id="(bench.projectVersionId as string)"
            :statement-cks="[panel.statementCk]"
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

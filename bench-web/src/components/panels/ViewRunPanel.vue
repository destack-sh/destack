<script lang="ts" setup>
import PanelHeader from "@/components/panels/PanelHeader.vue";
import { formatDuration, useTimeFromNow } from "@/composables/useNow";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type PanelContext, ViewRunPanel } from "@/state/bench";
import {
  TypeFlag,
  useCurrentModule,
  useNavigation,
  type NodeBase,
  type InterpStatement,
  type Statement,
} from "@/state/module";
import { computed, ref } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import { RUN_STATUS_NAME, getRunStatusColor, getRunStatusIconSolid, useCurrentSessions, useRun } from "@/state/session";
import { useTiling } from "@/state/screen";
import { getUUIDFromGlobalID } from "@/utils/functools";
import ContainerTile from "@/components/tiles/ContainerTile.vue";
import LogsTile from "@/components/tiles/LogsTile.vue";
import TraceTile from "@/components/tiles/TraceTile.vue";
import { getStatementIconSolid } from "@/state/statement";
import { TRIGGER_ICONS_SOLID } from "@/state/trigger";
import { IS_DEBUG } from "@/utils/globals";
import { StatementType, TriggerType } from "@/gql/graphql";
import { DateTime } from "luxon";
import ErrorTraceback from "@/components/basic/ErrorTraceback.vue";
import type { Run } from "@/gql/graphql";
import StructInterface from "@/components/interfaces/StructInterface.vue";
import PanelStatusNotice from "@/components/panels/PanelStatusNotice.vue";
import RunCacheInfo from "@/components/tiles/RunCacheInfo.vue";
import RunControlsTile from "@/components/tiles/RunControlsTile.vue";
import MonacoEditor from "@/components/basic/MonacoEditor.vue";

const props = defineProps<{ panel: PanelContext<ViewRunPanel>; focused: boolean }>();
const emit = defineEmits<{
  (e: "close"): void;
}>();

const bench = useBenchState();
const appearance = useAppearance();
const panel = computed(() => props.panel.panel.value);
const panelSize = computed(() => props.panel.size.value);
const now = useTimeFromNow(1000);
const nav = useNavigation();
const module = useCurrentModule();
const sessions = useCurrentSessions();
const codeKey = computed(() => module.runMetadataKey("code"));

// state

const runUuid = getUUIDFromGlobalID(panel.value.runId);
const { run: remoteRun, loading: remoteLoading } = useRun(
  computed(() => panel.value.runId),
  { live: true }
);
const localRun = computed(() => sessions.activeRuns.value.find((r) => r.id == panel.value.runId));
const run = computed(() => (remoteRun.value != null ? remoteRun.value : localRun.value));
const loading = computed(() => localRun.value == null && remoteLoading.value);
const statement = computed(() => (run.value?.statementCk != null ? module.statementOf(run.value.statementCk) : null));
const inputFields = computed(
  () => statement.value?.fields?.filter((t) => t.deletedAt == null && !(t.flags & TypeFlag.IS_OUTPUT)) ?? []
);
const outputFields = computed(
  () => statement.value?.fields?.filter((t) => t.deletedAt == null && t.flags & TypeFlag.IS_OUTPUT) ?? []
);
const terminalActions = computed(() => []);

const logsTileRef = ref<InstanceType<typeof LogsTile> | null>(null);
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
      :thing="run"
      :actions="terminalActions"
      :path="[
        { __typename: 'Panel', name: 'Runs' },
        { __typename: 'Run', id: panel.runId, name: `Run #${runUuid.slice(-7, -1)}` },
      ]"
      :self="1"
      @focus="
        (e) => {
          if (e.name == 'Runs') {
            bench.openViewRuns(undefined, { focus: true });
          }
        }
      "
    />
    <!-- Loading / status -->
    <div
      v-if="loading"
      class="flex h-full w-full flex-col items-center justify-center"
      :style="{
        width: props.panel.size.value?.width + 'px',
        height: props.panel.size.value?.height + 'px',
      }"
    >
      <BusySpinnerIcon class="mx-auto h-8 w-8 animate-spin text-gray-700" />
    </div>
    <PanelStatusNotice :thing="run" name="run" :loading="loading" />

    <!-- Tiles -->
    <div
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
        <h1 class="text-3xl font-bold text-gray-900">Run #{{ runUuid.slice(-7, -1) }}&nbsp;</h1>
        <RunControlsTile
          :run="(run as Run | undefined)"
          :statement="statement ?? undefined"
          :hide="['run']"
          @rerun="bench.openViewRun($event, { group: panel.group, focus: true })"
        />
      </div>
      <div v-if="module.loading.value" class="flex w-full flex-1 flex-col items-center justify-center">
        <BusySpinnerIcon class="mx-auto h-8 w-8 animate-spin text-white" />
      </div>
      <template v-else-if="run != null">
        <!-- Metadata -->
        <div class="flex flex-row flex-wrap gap-x-5 gap-y-2.5 px-2" :style="{ ...baseTilePositionX }">
          <!-- TODO :Cleanup: these elements are mostly copied (almost) verbatim from RunsTile -->
          <!-- Statement -->
          <div class="flex flex-col gap-0.5">
            <span class="text-xs font-semibold text-gray-500">Statement</span>
            <button
              v-if="statement != null"
              class="flex flex-row items-center whitespace-nowrap underline-offset-2 hover:underline"
              @click="nav.focusStatement(statement as NodeBase)"
            >
              <component
                :is="getStatementIconSolid((statement as InterpStatement).type)"
                class="mr-1 h-4 w-4 text-gray-400"
              />
              <span>{{ (statement as InterpStatement).name }}</span>
            </button>
            <span v-else class="text-gray-500">(not found)</span>
          </div>
          <!-- Status -->
          <div class="flex flex-col gap-0.5">
            <span class="text-xs font-semibold text-gray-500">Status</span>
            <span
              class="inline-flex flex-row items-center gap-1 whitespace-nowrap"
              :class="[getRunStatusColor(run.status)]"
            >
              <component
                :is="getRunStatusIconSolid(run.status)"
                class="h-4 w-4"
                :class="getRunStatusIconSolid(run.status) == BusySpinnerIcon ? 'animate-spin' : ''"
              />
              <span>{{ RUN_STATUS_NAME[run.status] }}</span>
              <!-- Duration -->
              <span v-if="run.startedAt != null">
                {{ run.terminatedAt != null ? "in" : "for" }}
                {{
                  run.duration != null
                    ? formatDuration(run.duration * 1000)
                    : now.getTimeFromNowString(run.startedAt, { useNow: false })
                }}
              </span>
              <RunCacheInfo :run="(run as Run)" class="px-0.5" />
            </span>
          </div>
          <!-- Trigger -->
          <div class="flex flex-col gap-0.5">
            <span class="text-xs font-semibold text-gray-500">Trigger</span>
            <div class="flex flex-row items-center gap-1 whitespace-nowrap">
              <!-- Type -->
              <component :is="TRIGGER_ICONS_SOLID[run.triggerType ?? TriggerType.Time]" class="h-4 w-4 text-gray-400" />
              <!-- From -->
              <span class="text-gray-900">{{ now.getTimeFromNowString(run.startedAt ?? run.createdAt) }}</span>
              <!-- Detail -->
              <span class="text-gray-900">
                <span v-if="run.triggerUser != null">by {{ run.triggerUser.username }}</span>
                <span v-else-if="run.triggerAccessToken != null">via API</span>
                <span v-else-if="run.parent != null">
                  in
                  <button
                    class="underline-offset-2 hover:underline"
                    @click.stop="bench.openViewRun(run.parent, { focus: true })"
                  >
                    #{{ getUUIDFromGlobalID(run.parent.id).slice(-7, -1) }}
                  </button>
                </span>
                <span v-else-if="run.trigger != null">by {{ run.trigger.type.toLowerCase() }} trigger</span>
                <span v-else-if="IS_DEBUG" class="text-red-600">???</span>
              </span>
            </div>
          </div>
          <!-- Run ID -->
          <div class="flex flex-col gap-0.5">
            <span class="text-xs font-semibold text-gray-500">Run ID</span>
            <span class="select-all font-mono text-gray-900">
              {{ getUUIDFromGlobalID(run.id) }}
            </span>
          </div>
          <!-- Last updated -->
          <div class="flex flex-col gap-0.5" v-if="run.updatedAt != null">
            <span class="text-xs font-semibold text-gray-500">Updated</span>
            <span class="text-gray-900">
              {{ DateTime.fromISO(run.updatedAt).toLocaleString(DateTime.DATETIME_MED_WITH_SECONDS) }}
            </span>
          </div>
        </div>
        <!-- Code (if available) -->
        <ContainerTile v-if="run.value?.[codeKey ?? ''] != null" label="Code" :style="{ ...baseTilePositionX }">
          <MonacoEditor
            :model-value="run.value[codeKey ?? '']"
            readonly
            wrap
            hide-line-numbers
            language="python"
            :focused="false"
            class="px-1"
          />
        </ContainerTile>
        <!-- Input -->
        <ContainerTile v-if="statement != null" label="Input" :style="{ ...baseTilePositionX }">
          <span v-if="inputFields?.length == 0" class="w-full text-center text-gray-400">No inputs</span>
          <StructInterface
            :model-value="run.inputs ?? {}"
            :type="(statement as Statement)"
            :is-output="false"
            full-inputs
            show-controls
            readonly
            :appearance="{ minimalFields: true, hideFieldType: true, view: 'tree' }"
          />
        </ContainerTile>
        <!-- Output -->
        <ContainerTile
          v-if="run.errorNice == null && statement != null"
          label="Output"
          :style="{ ...baseTilePositionX }"
        >
          <span v-if="outputFields?.length == 0" class="w-full text-center text-gray-400">No outputs</span>
          <StructInterface
            :model-value="run.outputs ?? {}"
            :type="(statement as Statement)"
            :is-output="true"
            full-inputs
            show-controls
            readonly
            :appearance="{ minimalFields: true, hideFieldType: true, view: 'tree' }"
          />
        </ContainerTile>
        <!-- Error -->
        <ContainerTile v-else-if="run.errorNice != null" label="Error" :style="{ ...baseTilePositionX }">
          <ErrorTraceback :statement-ck="run.statementCk" :error-nice="run.errorNice" class="p-1" />
        </ContainerTile>
        <!-- Trace -->
        <!-- TODO :Performance: pass in run to trace tiles (they all use the same data) -->
        <ContainerTile label="Trace" :style="{ ...baseTilePositionX }">
          <TraceTile :root-id="panel.runId" layout="list" live />
        </ContainerTile>
        <!-- Flamegraph (hidden because it's not very useful right now) -->
        <!-- <ContainerTile label="Flamegraph" :style="{ ...baseTilePositionX }">
          <TraceTile :root-id="panel.runId" layout="bars" live />
        </ContainerTile> -->
        <!-- Logs -->
        <ContainerTile v-if="statement?.type != StatementType.Model" label="Logs" :style="{ ...baseTilePositionX }">
          <LogsTile
            class="max-h-[500px] overflow-auto px-1 py-1"
            ref="logsTileRef"
            :project-id="(bench.projectId as string)"
            :project-version-id="(bench.projectVersionId as string)"
            :session-id="run.session?.id"
            lowlight
            :focus="{
              runId: panel.runId,
            }"
            :limit="100"
          />
        </ContainerTile>
      </template>
    </div>
  </div>
</template>

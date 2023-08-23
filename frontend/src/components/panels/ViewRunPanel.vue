<script lang="ts" setup>
import PanelHeader from "@/components/panels/PanelHeader.vue";
import RunsTile from "@/components/tiles/RunsTile.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type PanelContext, ViewRunPanel } from "@/state/bench";
import { TypeFlag, useCurrentModule } from "@/state/module";
import { computed, ref } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import { useCurrentSessions, useRun } from "@/state/session";
import { useTiling } from "@/state/screen";
import { getUUIDFromGlobalID } from "@/utils/functools";
import ContainerTile from "@/components/tiles/ContainerTile.vue";
import StructTile from "@/components/tiles/StructTile.vue";
import LogsTile from "@/components/tiles/LogsTile.vue";
import TraceTile from "@/components/tiles/TraceTile.vue";

const props = defineProps<{ panel: PanelContext<ViewRunPanel>; focused: boolean }>();
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
const runUuid = getUUIDFromGlobalID(panel.value.runId);
const { run } = useRun(
  computed(() => panel.value.runId),
  { live: true }
);
const statement = computed(() => (run.value?.runnable != null ? module.statementOf(run.value.runnable.id) : null));
const inputFields = computed(() => statement.value?.fields?.filter((t) => !(t.flags & TypeFlag.IsOutput)) ?? []);
const outputFields = computed(() => statement.value?.fields?.filter((t) => t.flags & TypeFlag.IsOutput) ?? []);
const terminalActions = computed(() => []);

const runsTileRef = ref<InstanceType<typeof RunsTile> | null>(null);
const logsTileRef = ref<InstanceType<typeof LogsTile> | null>(null);
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
      :path="[
        { __typename: 'Panel', name: 'Runs' },
        { __typename: 'Run', id: panel.runId, name: `Run #${runUuid.slice(-6, -1)}` },
      ]"
      :self="1"
      @focus="
        (e) => {
          if (e.name == 'Runs') {
            bench.openRuns(undefined, { focus: true });
          }
        }
      "
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
        <h1 class="text-3xl font-bold text-gray-900">Run: #{{ runUuid.slice(-6, -1) }}&nbsp;</h1>
      </div>
      <div v-if="module.loading.value" class="flex w-full flex-1 flex-col items-center justify-center">
        <BusySpinnerIcon class="mx-auto h-8 w-8 animate-spin text-white" />
      </div>
      <template v-else-if="run != null">
        <!-- Metadata -->
        <!-- nocheckin: run metadata -->
        <!-- Input -->
        <ContainerTile label="Input" :style="{ ...baseTilePositionX }">
          <span v-if="inputFields?.length == 0" class="w-full text-center text-gray-400">No inputs</span>
          <StructTile :model-value="run.inputs ?? {}" :fields="inputFields" full-inputs readonly class="" />
        </ContainerTile>
        <!-- Output -->
        <ContainerTile label="Output" :style="{ ...baseTilePositionX }">
          <span v-if="outputFields?.length == 0" class="w-full text-center text-gray-400">No outputs</span>
          <StructTile :model-value="run.outputs ?? {}" :fields="outputFields" full-inputs readonly class="" />
        </ContainerTile>
        <!-- Trace -->
        <ContainerTile label="Trace" :style="{ ...baseTilePositionX }">
          <TraceTile :root-id="panel.runId" layout="list" live />
        </ContainerTile>
        <!-- Logs -->
        <ContainerTile label="Logs" :style="{ ...baseTilePositionX }">
          <span v-if="logsTileRef != null && logsTileRef.logs?.length == 0" class="w-full text-center text-gray-400">
            No logs
          </span>
          <LogsTile
            ref="logsTileRef"
            :project-id="(bench.projectId as string)"
            :project-version-id="(bench.projectVersionId as string)"
            :run-id="panel.runId"
            :session-id="run.session?.id"
            :limit="100"
          />
        </ContainerTile>
      </template>
    </div>
  </div>
</template>

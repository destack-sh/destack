<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import PanelHeader from "@/components/panels/PanelHeader.vue";
import ContainerTile from "@/components/tiles/ContainerTile.vue";
import RunsTile from "@/components/tiles/RunsTile.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type ViewRunsPanel, type PanelContext } from "@/state/bench";
import { useCurrentModule } from "@/state/module";
import { useTiling } from "@/state/screen";
import { useCurrentSessions } from "@/state/session";
import { computed } from "vue";

const RUNS_PAGE_SIZE = 50;

const props = defineProps<{ panel: PanelContext<ViewRunsPanel>; focused: boolean }>();
const emit = defineEmits<{ (e: "close"): void }>();

const bench = useBenchState();
const appearance = useAppearance();
const panel = computed(() => props.panel.panel.value);
const panelSize = computed(() => props.panel.size.value);
const now = useTimeFromNow();

const module = useCurrentModule();
const sessions = useCurrentSessions();

const { gridStepX, gridStepY, getTileWidth, baseTilePositionX } = useTiling(props.panel);
</script>
<template>
  <div class="relative flex flex-col" :style="{ minHeight: panelSize.height + 'px' }">
    <PanelHeader
      class="border-b border-orange-900 border-opacity-[12%]"
      :thing="panel"
      :actions="[]"
      :editing="false"
      :readonly="bench.readonly"
      :path="[{ __typename: 'Panel', name: 'Runs' }]"
      :self="0"
    />
    <div v-if="module.loading.value" class="flex w-full flex-1 flex-col items-center justify-center">
      <BusySpinnerIcon class="mx-auto h-8 w-8 animate-spin text-white" />
    </div>
    <div
      v-else
      class="relative flex h-full w-full flex-col gap-6 px-2"
      :class="appearance.baseClass"
      :style="{
        marginTop: appearance.editorHeaderHeight + 'px',
        paddingTop: gridStepY + 'px',
        paddingBottom: gridStepY + 'px',
        minHeight: panelSize.height - appearance.editorHeaderHeight + 'px',
      }"
    >
      <!-- Filters -->
      <ContainerTile>
        <div class="flex flex-row gap-2">
          <!-- Status -->
          <!-- Runnable -->
          <!-- Time -->
        </div>
      </ContainerTile>
      <!-- Runs grid -->
      <ContainerTile>
        <!-- Header with pagination -->
        <div class="flex flex-row justify-between">
          <div>Runs</div>
          <!-- Pagination -->
          <div>pagination</div>
        </div>
        <!-- Results -->
        <!-- <RunsTile
        view="grid" /> -->
      </ContainerTile>
    </div>
  </div>
</template>

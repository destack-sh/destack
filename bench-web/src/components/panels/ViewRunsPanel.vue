<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import PanelHeader from "@/components/panels/PanelHeader.vue";
import ContainerTile from "@/components/tiles/ContainerTile.vue";
import RunsTile from "@/components/tiles/RunsTile.vue";
import { humanizeNumber, useTimeFromNow } from "@/composables/useNow";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type ViewRunsPanel, type PanelContext } from "@/state/bench";
import { useCurrentModule } from "@/state/module";
import { useTiling } from "@/state/screen";
import { useCurrentSessions } from "@/state/session";
import { ArrowLeftIcon, ArrowRightIcon } from "@heroicons/vue/24/outline";
import { computed, ref, type Ref } from "vue";

const props = defineProps<{ panel: PanelContext<ViewRunsPanel>; focused: boolean }>();
const emit = defineEmits<{ (e: "close"): void }>();

const bench = useBenchState();
const appearance = useAppearance();
const panel = computed(() => props.panel.panel.value);
const panelSize = computed(() => props.panel.size.value);
const now = useTimeFromNow();

const module = useCurrentModule();
const sessions = useCurrentSessions();

const runsTileRef: Ref<InstanceType<typeof RunsTile> | null> = ref(null);

// basic pagination that just remembers the last cursors
// for something more proper we'll need to reverse the sort and such
const runsAfter = ref<string | undefined>(undefined);
const previousCursors = ref<(string | null)[]>([]);
function pageForward() {
  previousCursors.value.push(runsAfter.value ?? null);
  runsAfter.value = runsTileRef.value?.pageInfo?.endCursor ?? undefined;
}

function pageBackward() {
  runsAfter.value = previousCursors.value.pop() ?? undefined;
}

const { gridStepX, gridStepY, getTileWidth, baseTilePositionX } = useTiling(props.panel);
</script>
<template>
  <div class="relative flex flex-col" :style="{ minHeight: panelSize.height + 'px' }">
    <PanelHeader
      class="border-b border-orange-900/[12%] bg-gray-50"
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
        marginTop: appearance.panelHeaderHeight + 'px',
        paddingTop: gridStepY + 'px',
        paddingBottom: gridStepY + 'px',
        minHeight: panelSize.height - appearance.panelHeaderHeight + 'px',
      }"
    >
      <!-- Header -->
      <div class="z-[1] flex flex-row items-baseline justify-between p-2" :style="baseTilePositionX">
        <!-- Title & source -->
        <h1 class="text-3xl font-bold text-gray-900">Runs</h1>
      </div>
      <!-- TODO @UX: runs panel filters (status/statement/query/etc.) -->
      <!-- Runs grid -->
      <ContainerTile
        :label="runsTileRef?.totalCount == null ? `Runs` : `${humanizeNumber(runsTileRef.totalCount)} runs`"
        :style="{ ...baseTilePositionX }"
        sublabel-position="opposite"
        class="overflow-x-hidden"
      >
        <!-- Pagination  -->
        <template v-slot:sublabel>
          <span class="flex flex-row items-center gap-1 font-normal text-gray-500">
            <!-- Navigate backward -->
            <button
              class="p-0.5"
              :class="[
                previousCursors.length > 0 ? 'text-gray-400 hover:bg-orange-100 hover:text-gray-900' : 'text-gray-200',
              ]"
              :disabled="previousCursors.length === 0"
              @click="pageBackward()"
            >
              <ArrowLeftIcon class="h-3 w-3 text-gray-700" />
            </button>
            <span
              >page {{ previousCursors.length + 1 }} of
              {{ Math.ceil((runsTileRef?.totalCount ?? 0) / panel.limit) }}</span
            >
            <!-- Navigate forward -->
            <button
              class="p-0.5"
              :class="[
                runsTileRef?.pageInfo?.hasNextPage
                  ? 'text-gray-400 hover:bg-orange-100 hover:text-gray-900'
                  : 'text-gray-200',
              ]"
              :disabled="!runsTileRef?.pageInfo?.hasNextPage"
              @click="pageForward()"
            >
              <ArrowRightIcon class="h-3 w-3 text-gray-700" />
            </button>
          </span>
        </template>
        <!-- Results -->
        <RunsTile
          ref="runsTileRef"
          hide-header
          :project-id="(bench.projectId as string)"
          :project-version-id="(bench.projectVersionId as string)"
          live
          :after="runsAfter"
          :limit="panel.limit"
          root-only
        />
      </ContainerTile>
    </div>
  </div>
</template>

<script lang="ts" setup>
import ErrorTraceback from "@/components/basic/ErrorTraceback.vue";
import LogsTile from "@/components/tiles/LogsTile.vue";
import RunMetadataTile from "@/components/tiles/RunMetadataTile.vue";
import TraceTile from "@/components/tiles/TraceTile.vue";
import { RunStatus, type Run, type LogEntry } from "@/gql/graphql";
import { Bars3Icon, ChartBarIcon, DocumentChartBarIcon, FireIcon, XCircleIcon } from "@heroicons/vue/24/outline";
import { ref, watch } from "vue";

type View = "logs" | "flamegraph" | "trace" | "error" | "metadata";

const props = defineProps<{
  projectId: string;
  projectVersionId?: string | null;
  run: Run;
  view?: View;
  showControls?: boolean;
}>();

const logsTileRef = ref<InstanceType<typeof LogsTile> | null>(null);

const activeView = ref<View>(props.view ?? "logs");

// sync props view into activeView on change
watch(
  () => props.view,
  (view) => {
    if (view != null) activeView.value = view;
  }
);

defineExpose({
  addLogs(logs: LogEntry[]) {
    logsTileRef.value?.addLogs(logs);
  },
});
</script>
<template>
  <div class="relative">
    <!-- TODO @UX: run tile is ugly af -->
    <!-- Controls -->
    <div v-if="showControls" class="flex flex-row items-center justify-between">
      <span class="text-xs font-semibold uppercase text-gray-400">{{ activeView }}</span>
      <!-- View switcher -->
      <div class="group/controls z-10 flex flex-row gap-1">
        <button
          v-for="view in ['logs', 'trace', 'flamegraph', 'error', 'metadata'].filter(
            (v) => v != 'error' || run?.status == RunStatus.Failed
          )"
          :key="view"
          class="group/button relative cursor-pointer rounded-sm p-0.5 hover:bg-orange-100"
          :class="[activeView == view ? 'text-orange-600' : 'text-gray-400 hover:text-gray-700']"
          @click="activeView = view as View"
        >
          <component
            :is="
              {
                logs: Bars3Icon,
                flamegraph: FireIcon,
                trace: ChartBarIcon,
                error: XCircleIcon,
                metadata: DocumentChartBarIcon,
              }[view]
            "
            class="h-4 w-4"
          />
          <!-- Label -->
          <span
            class="pointer-events-none absolute -left-8 top-6 z-10 whitespace-nowrap rounded-sm border border-orange-900 border-opacity-[15%] bg-white px-2 py-0.5 text-center text-xs text-gray-500 opacity-0 transition duration-150 group-hover/button:opacity-100"
          >
            Show {{ view }}
          </span>
        </button>
      </div>
    </div>
    <!-- View container (scrollable) -->
    <div ref="outputRef" class="mt-1">
      <!-- Output views -->
      <TraceTile
        v-if="activeView == 'flamegraph' || activeView == 'trace'"
        :session-id="run.session?.id"
        :root-id="run.id"
        :layout="activeView == 'flamegraph' ? 'bars' : 'list'"
        live
      />
      <LogsTile
        v-else-if="activeView == 'logs' && run.session?.id != null"
        ref="logsTileRef"
        class="max-h-[300px] overflow-auto"
        :containerHeight="300"
        :project-id="(projectId as string)"
        :project-version-id="(projectVersionId as string)"
        :session-id="run.session?.id"
        :focus="{
          runnableIds: run.runnable != null ? [run.runnable.id] : undefined,
        }"
        lowlight
        live
        :limit="500"
      />
      <ErrorTraceback v-else-if="activeView == 'error'" :run="run" />
      <RunMetadataTile v-else-if="activeView == 'metadata'" :run="run" />
    </div>
  </div>
</template>

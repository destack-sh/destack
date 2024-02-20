<script lang="ts" setup>
import ErrorTraceback from "@/components/basic/ErrorTraceback.vue";
import LogsTile from "@/components/tiles/LogsTile.vue";
import TraceTile from "@/components/tiles/TraceTile.vue";
import { RunStatus, type Run, type LogEntry } from "@/gql/graphql";
import { Bars3Icon, ChartBarIcon, FireIcon, QueueListIcon, XCircleIcon, XMarkIcon } from "@heroicons/vue/24/outline";
import { ref, watch } from "vue";

type View = "logs" | "error" | "trace";

const props = defineProps<{
  projectId: string;
  projectVersionId?: string | null;
  run: Run;
  view?: View;
  showControls?: boolean;
  showClose?: boolean;
}>();
const emit = defineEmits<{
  (e: "close"): void;
}>();

const VIEW_ICONS: Record<string, any> = {
  logs: Bars3Icon,
  flamegraph: FireIcon,
  trace: QueueListIcon,
  error: XCircleIcon,
};
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
          v-for="view in ['logs', 'error', 'trace'].filter((v) => v != 'error' || run?.status == RunStatus.Failed)"
          :key="view"
          class="group/button relative cursor-pointer rounded-sm p-0.5 hover:bg-orange-100"
          :class="[activeView == view ? 'text-orange-600' : 'text-gray-400 hover:text-gray-700']"
          @click="activeView = (view as View)"
        >
          <component :is="VIEW_ICONS[view]" class="h-4 w-4" />
          <!-- Label -->
          <span
            class="pointer-events-none absolute -left-8 top-6 z-10 whitespace-nowrap rounded-sm border border-orange-900 border-opacity-[15%] bg-white px-2 py-0.5 text-center text-xs text-gray-500 opacity-0 transition duration-150 group-hover/button:opacity-100"
          >
            Show {{ view }}
          </span>
        </button>
        <!-- close -->
        <button
          v-if="showClose"
          class="group/button relative cursor-pointer rounded-sm p-0.5 text-gray-400 hover:bg-orange-100 hover:text-gray-700"
          @click="emit('close')"
        >
          <XMarkIcon class="h-4 w-4" />
          <!-- Label -->
          <span
            class="pointer-events-none absolute -left-8 top-6 z-10 whitespace-nowrap rounded-sm border border-orange-900 border-opacity-[15%] bg-white px-2 py-0.5 text-center text-xs text-gray-500 opacity-0 transition duration-150 group-hover/button:opacity-100"
          >
            Close
          </span>
        </button>
      </div>
    </div>
    <!-- View container (scrollable) -->
    <div ref="outputRef" class="mt-1">
      <!-- Output views -->
      <TraceTile v-if="activeView == 'trace'" :session-id="run.session?.id" :root-id="run.id" layout="list" live />
      <LogsTile
        v-else-if="activeView == 'logs' && run.session?.id != null"
        ref="logsTileRef"
        class="max-h-[300px] overflow-auto"
        :containerHeight="300"
        :project-id="(projectId as string)"
        :project-version-id="(projectVersionId as string)"
        :session-id="run.session?.id"
        :focus="{
          statementCks: run.statementCk != null ? [run.statementCk] : undefined,
        }"
        lowlight
        live
        :limit="500"
      />
      <ErrorTraceback
        v-else-if="activeView == 'error' && run.errorNice != null"
        :statement-ck="run.statementCk"
        :error-nice="run.errorNice"
      />
    </div>
  </div>
</template>

<script lang="ts" setup>
import FileExplorer from "@/components/views/FileExplorer.vue";
import StatementExplorer from "@/components/views/StatementExplorer.vue";
import { useActions } from "@/state/actions";
import { useAppearance } from "@/state/appearance";
import { useCurrentModule } from "@/state/module";
import { PlusIcon } from "@heroicons/vue/24/outline";
import { useElementBounding, useFocusWithin } from "@vueuse/core";
import { computed, ref, watch, type Component, type Ref } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import { useBenchState } from "@/state/bench";
import PanelExplorer from "@/components/views/PanelExplorer.vue";
import { useElementRefs } from "@/composables/useGrid";

const props = defineProps<{ active: boolean; focused: boolean }>();
const emit = defineEmits<{ (e: "show"): void; (e: "blur"): void }>();

const actions = useActions();
const appearance = useAppearance();
const module = useCurrentModule();
const bench = useBenchState();

type Explorer = {
  title: string;
  actions: Action[];
  component: Component;
};

type Action = {
  icon: Component;
  label: string;
  action: (symbol: Symbol) => void;
  enabled: boolean;
};

const explorers: Ref<Explorer[]> = computed(() => {
  const explorers: Explorer[] = [];
  if (bench.showPanelExplorer && bench.panels.length > 0) {
    explorers.push({
      title: "Panels",
      component: PanelExplorer,
      actions: [],
    });
  }
  explorers.push({
    title: "Files",
    component: FileExplorer,
    actions: [
      {
        icon: PlusIcon,
        label: "File",
        action: () => actions.file.create.value.apply(),
        enabled: actions.file.create.value.enabled,
      },
    ],
  } as Explorer);
  if (bench.focusedFileId != null) {
    explorers.push({
      title: "Outline",
      component: StatementExplorer,
      actions: [],
    } as Explorer);
  }
  return explorers;
});

const containerRef: Ref<HTMLDivElement | null> = ref(null);
const containerBounding = useElementBounding(containerRef);
const explorerRefs =
  useElementRefs<InstanceType<typeof PanelExplorer | typeof FileExplorer | typeof StatementExplorer>>();

function onNavigateUp(panel: Explorer) {
  const index = explorers.value.indexOf(panel);
  if (index > 0) {
    explorerRefs.getRef(explorers.value[index - 1].title)?.focus("last");
  }
}

function onNavigateDown(panel: Explorer) {
  const index = explorers.value.indexOf(panel);
  if (index < explorers.value.length - 1) {
    explorerRefs.getRef(explorers.value[index + 1].title)?.focus("first");
  }
}

// handle focus
const { focused: inContainerFocused } = useFocusWithin(containerRef);

// focus view when getting focus
watch(inContainerFocused, () => {
  if (inContainerFocused.value) {
    emit("show");
  } else {
    emit("blur");
  }
});
// handle explorer view focus and editor focus
watch(
  () => props.focused,
  () => {
    if (props.focused) {
      if (!inContainerFocused.value) {
        // start to focus files if nothing was directly focused
        explorerRefs.getRef("Files")?.focus();
      }
    } else {
      explorerRefs.refs.value.forEach((ref) => ref.blur());
    }
  },
  { immediate: true }
);
</script>
<template>
  <div ref="containerRef" class="relative flex h-full flex-col">
    <!-- View panels -->
    <div class="flex flex-1 flex-col gap-y-3 pb-10">
      <div v-for="explorer in explorers" :key="explorer.title" class="min-h-0">
        <!-- Explorer header -->
        <div
          class="flex flex-shrink-0 flex-row items-center justify-between px-3"
          :style="{
            height: appearance.panelHeaderHeight + 'px',
          }"
        >
          <span class="select-none text-xs font-semibold tracking-wide text-gray-500">
            {{ explorer.title }}
          </span>
          <!-- Explorer actions -->
          <div v-if="module.loading.value">
            <BusySpinnerIcon class="h-4 w-4 animate-spin text-gray-500" />
          </div>
          <span class="inline-flex flex-row gap-1" v-else>
            <button
              v-for="action in explorer.actions.filter((action) => action.enabled)"
              :key="action.label"
              class="inline-flex flex-row rounded-sm p-0.5 text-gray-400 transition duration-150 hover:bg-orange-100 hover:text-gray-700"
              @click.prevent="action.action"
            >
              <component :is="action.icon" class="h-4 w-4" />
              <span class="sr-only pl-0.5 text-xs text-gray-700">{{ action.label }}</span>
            </button>
          </span>
        </div>
        <!-- Explorer content -->
        <!-- TODO @UX: view sections are not sized properly if heights are unbalanced (e.g. in explorer view) -->
        <div
          class="scroll-hidden min-h-0 overflow-y-auto"
          :style="{
            maxHeight: containerBounding.height.value / explorers.length - appearance.panelHeaderHeight + 'px',
          }"
        >
          <component
            :is="explorer.component"
            :ref="(ref: any) => explorerRefs.registerRef(explorer.title, ref)"
            :focused="props.focused"
            @navigate-up="onNavigateUp(explorer)"
            @navigate-down="onNavigateDown(explorer)"
          />
        </div>
      </div>
    </div>
  </div>
</template>

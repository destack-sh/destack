<script lang="ts" setup>
import FileExplorer from "@/components/views/FileExplorer.vue";
import SymbolExplorer from "@/components/views/SymbolExplorer.vue";
import { useActions } from "@/state/actions";
import { useAppearance } from "@/state/appearance";
import { useCurrentModule } from "@/state/module";
import { PlusIcon } from "@heroicons/vue/24/outline";
import { useFocusWithin } from "@vueuse/core";
import { computed, ref, toRef, watch, type Component, type Ref } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";

const props = defineProps<{ focused: boolean }>();
const emit = defineEmits<{ (e: "show"): void; (e: "blur"): void }>();

const actions = useActions();
const appearance = useAppearance();
const module = useCurrentModule();

type Panel = {
  title: string;
  actions: Action[];
  count?: number;
};

type Action = {
  icon: Component;
  label: string;
  action: (symbol: Symbol) => void;
  enabled: boolean;
};

const panels: Ref<Panel[]> = computed(() => [
  {
    title: "Files",
    count: fileExplorer.value?.count,
    actions: [
      {
        icon: PlusIcon,
        label: "File",
        action: () => actions.file.create.value.apply(),
        enabled: actions.file.create.value.enabled,
      },
    ],
  } as Panel,
  {
    title: "Outline",
    count: symbolExplorer.value?.count,
    actions: [],
  } as Panel,
]);

const containerRef: Ref<HTMLDivElement | null> = ref(null);
const fileExplorer: Ref<InstanceType<typeof FileExplorer> | undefined> = ref(undefined);
const symbolExplorer: Ref<InstanceType<typeof SymbolExplorer> | undefined> = ref(undefined);

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
  toRef(props, "focused"),
  () => {
    if (props.focused) {
      if (!inContainerFocused.value) {
        // start to focus files if nothing was directly focused
        fileExplorer.value?.focus();
      }
    } else {
      fileExplorer.value?.blur();
      symbolExplorer.value?.blur();
    }
  },
  { immediate: true }
);
</script>
<template>
  <div ref="containerRef" class="relative flex h-full flex-col">
    <!-- View panels -->
    <div class="flex flex-1 flex-col gap-y-3">
      <div v-for="panel in panels" :key="panel.title" class="min-h-0">
        <!-- Panel header -->
        <div
          class="flex flex-shrink-0 flex-row items-center justify-between px-3"
          :style="{
            height: appearance.editorHeaderHeight + 'px',
          }"
        >
          <span class="select-none text-sm font-extrabold text-gray-500">
            {{ panel.title }}
          </span>
          <!-- Panel actions -->
          <div v-if="module.loading.value">
            <BusySpinnerIcon class="h-4 w-4 animate-spin text-gray-500" />
          </div>
          <span class="inline-flex flex-row gap-1" v-else>
            <button
              v-for="action in panel.actions.filter((action) => action.enabled)"
              :key="action.label"
              class="inline-flex flex-row rounded-sm p-0.5 hover:bg-orange-100 hover:text-gray-700"
              @click.prevent="action.action"
            >
              <component :is="action.icon" class="h-4 w-4 text-gray-400" />
              <span class="sr-only pl-0.5 text-xs text-gray-700">{{ action.label }}</span>
            </button>
          </span>
        </div>
        <!-- Panel content -->
        <div class="min-h-0 overflow-y-auto">
          <FileExplorer
            :ref="(ref) => (fileExplorer = ref as any)"
            v-if="panel.title == 'Files'"
            :focused="props.focused"
            @navigate-down="symbolExplorer?.focus('first')"
            @navigate-up="symbolExplorer?.focus('last')"
          />
          <SymbolExplorer
            :ref="(ref) => (symbolExplorer = ref as any)"
            v-else-if="panel.title == 'Outline'"
            :focused="props.focused"
            @navigate-up="fileExplorer?.focus('last')"
            @navigate-down="fileExplorer?.focus('first')"
          />
          <!-- <span v-else class="text-red-500">panic!</span> -->
        </div>
      </div>
    </div>
  </div>
</template>

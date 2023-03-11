<script lang="ts" setup>
import FileExplorer from "@/components/panels/FileExplorer.vue";
import SymbolExplorer from "@/components/panels/SymbolExplorer.vue";
import { useActions } from "@/state/actions";
import { useEditorState, type FileHeader } from "@/state/editor";
import { PlusIcon } from "@heroicons/vue/24/outline";
import { useFocusWithin } from "@vueuse/core";
import { computed, ref, toRef, watch, type Component, type Ref } from "vue";

const props = defineProps<{ files?: FileHeader[]; focused: boolean }>();
const emit = defineEmits<{ (e: "show"): void; (e: "blur"): void }>();

const actions = useActions();

type Panel = {
  title: string;
  actions: Action[];
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
    title: "Symbols",
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
  <div ref="containerRef">
    <!-- View header -->
    <div
      class="flex h-[31px] flex-row items-center justify-between border-b border-orange-900 border-opacity-[12%] px-3 py-2"
    >
      <span class="select-none text-xs font-bold uppercase">Explorer</span>
    </div>
    <!-- View panels -->
    <div class="flex flex-1 flex-col gap-y-2 divide-y divide-orange-900 divide-opacity-[12%]">
      <div v-for="panel in panels" :key="panel.title">
        <!-- Panel header -->
        <div class="flex flex-row items-center justify-between px-3 py-1">
          <span class="select-none text-xs font-bold uppercase">
            {{ panel.title }}
          </span>
          <!-- Panel actions -->
          <span class="inline-flex flex-row gap-1">
            <button
              v-for="action in panel.actions.filter((action) => action.enabled)"
              :key="action.label"
              class="inline-flex flex-row rounded-sm p-0.5 hover:bg-gray-100 hover:text-gray-700"
              @click.prevent="action.action"
            >
              <component :is="action.icon" class="h-4 w-4 text-gray-400" />
              <span class="sr-only pl-0.5 text-xs text-gray-700">{{ action.label }}</span>
            </button>
          </span>
        </div>
        <!-- Panel content -->
        <FileExplorer
          :ref="(ref) => (fileExplorer = ref as any)"
          v-if="panel.title == 'Files'"
          :files="props.files"
          @navigate-down="symbolExplorer?.focus()"
        />
        <SymbolExplorer
          :ref="(ref) => (symbolExplorer = ref as any)"
          v-else-if="panel.title == 'Symbols'"
          @navigate-up="fileExplorer?.focus()"
        />
        <span v-else class="text-red-500">panic!</span>
      </div>
    </div>
  </div>
</template>

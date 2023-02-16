<script lang="ts" setup>
import FileExplorer from "@/components/panels/FileExplorer.vue";
import SymbolExplorer from "@/components/panels/SymbolExplorer.vue";
import { useActions } from "@/state/actions";
import type { FileHeader } from "@/state/editor";
import { PlusIcon } from "@heroicons/vue/24/outline";
import type { Component } from "vue";

const props = defineProps<{ files: FileHeader[] }>();

const actions = useActions();

type Panel = {
  title: string;
  actions: Action[];
};

type Action = {
  icon: Component;
  label: string;
  action: (symbol: Symbol) => void;
};

const panels: Panel[] = [
  {
    title: "Files",
    actions: [
      {
        icon: PlusIcon,
        label: "File",
        action: () => actions.file.create.value.apply(),
      },
    ],
  },
  {
    title: "Symbols",
    actions: [],
  },
];
</script>
<template>
  <div ref="container">
    <!-- View header -->
    <div class="flex h-[31px] flex-row items-center justify-between border-b border-gray-200 px-3 py-2">
      <span class="text-xs font-bold uppercase">Explorer</span>
    </div>
    <!-- View panels -->
    <div class="flex flex-1 flex-col gap-y-2 divide-y divide-gray-200">
      <div v-for="panel in panels" :key="panel.title">
        <!-- Panel header -->
        <div class="flex flex-row items-center justify-between px-3 py-1">
          <span class="text-xs font-bold uppercase">
            {{ panel.title }}
          </span>
          <!-- Panel actions -->
          <span class="inline-flex flex-row gap-1">
            <button
              v-for="action in panel.actions"
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
        <FileExplorer v-if="panel.title == 'Files'" :files="props.files" />
        <SymbolExplorer v-else-if="panel.title == 'Symbols'" />
        <span v-else class="text-red-500">panic!</span>
      </div>
    </div>
  </div>
</template>

<script lang="ts" setup>
import FileExplorer from "@/components/views/FileExplorer.vue";
import StatementExplorer from "@/components/views/StatementExplorer.vue";
import { useActions } from "@/state/actions";
import { useAppearance } from "@/state/appearance";
import { useCurrentModule } from "@/state/module";
import { PlusIcon } from "@heroicons/vue/24/outline";
import { computed, ref, watch, type Component, type Ref } from "vue";
import { useBenchState, type Action } from "@/state/bench";
import PanelExplorer from "@/components/views/PanelExplorer.vue";
import ViewSectionGroup from "@/components/views/ViewSectionGroup.vue";
import ViewSection from "@/components/views/ViewSection.vue";

const props = defineProps<{ active: boolean; focused: boolean }>();
const emit = defineEmits<{ (e: "show"): void; (e: "blur"): void }>();

const actions = useActions();
const module = useCurrentModule();
const bench = useBenchState();

type Explorer = {
  title: string;
  actions: Action<unknown>[];
  component: Component;
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
        disabled: bench.readonly,
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
</script>
<template>
  <ViewSectionGroup :focused="props.focused" @show="emit('show')" @blur="emit('blur')">
    <!-- View panels -->
    <ViewSection
      v-for="(explorer, i) in explorers"
      :key="explorer.title"
      :index="i"
      :title="explorer.title"
      :actions="explorer.actions"
      :loading="module.loading.value"
      v-slot="{ navigateUp, navigateDown }"
    >
      <component
        :is="explorer.component"
        :focused="props.focused"
        @navigate-up="navigateUp"
        @navigate-down="navigateDown"
      />
    </ViewSection>
  </ViewSectionGroup>
</template>

<script lang="ts" setup>
import EditFilePanelInterface from "@/components/panels/EditFilePanel.vue";
import EditDatabasePanelInterface from "@/components/panels/EditDatabasePanel.vue";
import LaunchRunPanelInterface from "@/components/panels/LaunchRunPanel.vue";
import ViewRunsPanelInterface from "@/components/panels/ViewRunsPanel.vue";
import ViewRunPanelInterface from "@/components/panels/ViewRunPanel.vue";
import TerminalPanelInterface from "@/components/panels/TerminalPanel.vue";
import { useActiveScroll } from "@/composables/useScroll";
import { providePanelContext, useBenchState, type PanelContext, Panel, type PanelType } from "@/state/bench";
import { useEventListener } from "@vueuse/core";
import { computed, onBeforeUnmount, onMounted, ref, toRef } from "vue";

const bench = useBenchState();
const props = defineProps<{ panel: Panel; containerEl: HTMLElement | null }>();
const containerRef = ref<InstanceType<typeof EditFilePanelInterface> | null>(null);
const containerEl = toRef(props, "containerEl");
const focused = computed(() => bench.focusedPanelId == props.panel.id);

const componentsByPanel: Partial<Record<PanelType, any>> = {
  "edit-file": EditFilePanelInterface,
  "edit-database": EditDatabasePanelInterface,
  "launch-run": LaunchRunPanelInterface,
  "view-runs": ViewRunsPanelInterface,
  "view-run": ViewRunPanelInterface,
  terminal: TerminalPanelInterface,
};

const scroll = useActiveScroll(containerEl);
// auto focus on click
useEventListener(containerEl, "click", () => {
  bench.focusPanel(props.panel);
});

const context = providePanelContext(toRef(props, "panel"), containerRef, toRef(props, "containerEl"), scroll);

onMounted(() => {
  props.panel.onMounted?.(context);
});
onBeforeUnmount(() => {
  props.panel.onUnmounted?.();
});

defineExpose({
  context,
});
</script>
<template>
  <component
    v-if="componentsByPanel[panel.type] != null"
    :is="componentsByPanel[panel.type]"
    ref="containerRef"
    :panel="(context as PanelContext<any>)"
    :focused="focused"
    @close="bench.closePanel(panel)"
    @focus="bench.focusPanel(panel)"
  />
  <div v-else class="h-full w-full text-center">
    <span class="text-red-600">cannot render editor of type {{ panel.type }}</span>
  </div>
</template>

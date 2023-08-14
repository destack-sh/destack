<script lang="ts" setup>
import FilePanelInterface from "@/components/editors/FileEditor.vue";
import StatementPanelInterface from "@/components/editors/StatementEditor.vue";
import LaunchPanelInterface from "@/components/editors/LaunchPanel.vue";
import { useActiveScroll } from "@/composables/useScroll";
import {
  providePanelContext,
  useBenchState,
  type PanelContext,
  type FileEditor,
  StatementEditor,
  LaunchPanel,
  Panel,
} from "@/state/bench";
import { useEventListener } from "@vueuse/core";
import { computed, onBeforeUnmount, onMounted, ref, toRef } from "vue";

const bench = useBenchState();
const props = defineProps<{ panel: Panel; containerEl: HTMLElement | null }>();
const containerRef = ref<InstanceType<typeof FilePanelInterface> | null>(null);
const containerEl = toRef(props, "containerEl");
const focused = computed(() => bench.focusedPanelId == props.panel.id);

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
  <FilePanelInterface
    ref="containerRef"
    v-if="panel.type == 'file'"
    :panel="(context as PanelContext<FileEditor>)"
    :fileId="(panel as FileEditor).fileId"
    :focused="focused"
    @close="bench.closePanel(panel)"
  />
  <StatementPanelInterface
    ref="containerRef"
    v-else-if="panel.type == 'statement'"
    :panel="(context as PanelContext<StatementEditor>)"
    :focused="focused"
    @close="bench.closePanel(panel)"
  />
  <LaunchPanelInterface
    ref="containerRef"
    v-else-if="panel.type == 'launch'"
    :panel="(context as PanelContext<LaunchPanel>)"
    :focused="focused"
    @close="bench.closePanel(panel)"
  />
  <div v-else class="h-full w-full text-center">
    <span class="text-red-600">cannot render editor of type {{ panel.type }}</span>
  </div>
</template>

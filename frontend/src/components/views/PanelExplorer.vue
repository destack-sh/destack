<script lang="ts" setup>
import { useNavigationGrid } from "@/composables/useGrid";
import { Panel, PANEL_ICONS_OUTLINE, PANEL_ICONS_SOLID, useBenchState, type ViewId } from "@/state/bench";
import { useFocusWithin } from "@vueuse/core";
import { computed, nextTick, ref, type Ref } from "vue";

const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
}>();

const bench = useBenchState();
const panelsSorted = computed(() =>
  bench.panels.slice().sort((a, b) => (b.lastFocusedAt ?? "").localeCompare(a.lastFocusedAt ?? ""))
);
const focusedPanelId = computed(() => panelsSorted.value.find((f) => f.id == bench.focusedPanelId)?.id);
const panelsGrid = useNavigationGrid<"name", HTMLElement>(
  computed(() => ["name"]),
  panelsSorted,
  {
    gridNavigateUp: () => emit("navigateUp"),
    gridNavigateDown: () => emit("navigateDown"),
  }
);

function focusPanel(panel: Panel) {
  const focusedViewId = bench.focusedViewId;
  bench.focusPanel(panel);
  bench.focusView(focusedViewId as ViewId); // keep focused view
}

function focusPanelAndGoThere(panel: Panel) {
  bench.focusPanel(panel);
}

// blur focused panel if clicking outside panel explorer
const listRef: Ref<HTMLDivElement | null> = ref(null);
const { focused: listRefFocused } = useFocusWithin(listRef);

function focus(target?: "first" | "last") {
  // focus currently focused panel if nothing was directly selected (and thus focused)
  if (!target && focusedPanelId.value != null && !listRefFocused.value) {
    nextTick(() => panelsGrid.focus(focusedPanelId.value as string, "name"));
  } else if (!listRefFocused.value && (panelsSorted.value.length ?? 0) > 0) {
    nextTick(() => panelsGrid.focus(target == "first" ? 0 : -1, "name"));
  }
}

function blur() {
  panelsGrid.blur();
}

defineExpose({
  count: computed(() => panelsSorted.value.length),
  focus,
  blur,
});
</script>
<template>
  <!-- Panel: panel explorer -->
  <ul ref="listRef" role="list" class="flex flex-col text-sm">
    <li
      v-for="panel in panelsSorted"
      :key="panel.id"
      :ref="(ref) => panelsGrid.registerColumnRef(panel.id, 'name', (ref as HTMLElement))"
      tabindex="-1"
      @keydown.up.exact.prevent="panelsGrid.navigateUp(panel.id, 'name')"
      @keydown.down.exact.prevent="panelsGrid.navigateDown(panel.id, 'name')"
      class="relative max-w-full border border-transparent px-3 py-0.5 outline-none hover:cursor-pointer hover:bg-orange-100 focus:border-orange-600"
      :class="{
        'text-orange-600': panel.id == bench?.focusedPanelId,
        'text-gray-700 hover:text-orange-600': panel.id != bench?.focusedPanelId,
      }"
      @click="focusPanel(panel)"
      @keydown.enter.exact.prevent="focusPanelAndGoThere(panel)"
    >
      <!-- Panel info -->
      <span class="flex flex-row items-center">
        <component :is="PANEL_ICONS_SOLID[panel.type]" class="mr-1.5 h-4 w-4 text-gray-500" />
        <span
          class="decoration-none inline select-none truncate text-ellipsis rounded-sm text-sm placeholder-gray-400 outline-none"
        >
          {{ panel.name.length > 0 ? panel.name : "(Unnamed)" }}
        </span>
      </span>
    </li>
  </ul>
</template>

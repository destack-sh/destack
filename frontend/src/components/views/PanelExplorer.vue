<script lang="ts" setup>
import { useElementRefs } from "@/composables/useGrid";
import { Panel, PANEL_ICONS_SOLID, useBenchState, type ViewId } from "@/state/bench";
import { useFocusWithin } from "@vueuse/core";
import { computed, nextTick, ref, type Ref } from "vue";

const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
}>();

const bench = useBenchState();
const groups = computed(() => bench.groups.filter((g) => g.panels.length > 0));
const panelRefs = useElementRefs<HTMLElement>();

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
  if (!target && bench.focusedPanelId != null && !listRefFocused.value) {
    nextTick(() => panelRefs.focus(bench.focusedPanelId as string));
  } else if (!listRefFocused.value && (bench.panels.length ?? 0) > 0) {
    if (target == "first") {
      nextTick(() => panelRefs.focus(bench.panels[0].id));
    } else {
      nextTick(() => panelRefs.focus(bench.panels[bench.panels.length - 1].id));
    }
  }
}

function blur() {
  panelRefs.refs.value.forEach((ref) => ref.blur());
}

defineExpose({
  count: computed(() => bench.panels.length),
  focus,
  blur,
});
</script>
<template>
  <!-- Panel: panel explorer -->
  <div class="flex flex-col gap-2">
    <!-- Each group -->
    <ul ref="listRef" role="list" class="flex flex-col text-sm" v-for="group in groups" :key="group.id">
      <h3 v-if="groups.length > 1" class="px-3 text-xs font-semibold text-gray-500">
        {{ group.name }}
      </h3>
      <!-- Each panel -->
      <li
        v-for="panel in group.panels"
        :key="panel.id"
        :ref="(ref: any) => panelRefs.registerRef(panel.id, ref)"
        tabindex="-1"
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
          <component :is="PANEL_ICONS_SOLID[panel.type]" class="mr-1.5 h-4 w-4" />
          <span
            class="decoration-none inline select-none truncate text-ellipsis rounded-sm text-sm placeholder-gray-400 outline-none"
          >
            {{ panel.name.length > 0 ? panel.name : "(Unnamed)" }}
          </span>
        </span>
      </li>
    </ul>
  </div>
</template>

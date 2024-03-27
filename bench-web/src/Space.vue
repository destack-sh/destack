<script lang="ts" setup>
import { NodeType, Orientation, ViewData, ViewType } from "@/proto/wire";
import { useActiveConnection } from "@/system/connection";
import { LOCAL_SPACE_PTR, spacePtr } from "@/system/client";
import { isDragging } from "@/utils/layout";
import Bar from "@/views/private/Bar.vue";
import Omnibar from "@/views/private/Omnibar.vue";
import ToastOverlay from "@/views/private/ToastOverlay.vue";
import TooltipOverlay from "@/views/private/TooltipOverlay.vue";
import MenuOverlay from "@/views/private/MenuOverlay.vue";
import { useTitle, useWindowSize } from "@vueuse/core";
import { computed, ref, watch } from "vue";
import { toNodeReference } from "@/proto/wiring";
import Split from "@/views/containers/Split.vue";
import { bench, canvas } from "@/system/space";

const BAR_HEIGHT = 42;
const BAR_OFFSET = 0;
const spaceRef = ref<HTMLElement | null>(null);
const barRef = ref<InstanceType<typeof Bar> | null>(null);
const { width: spaceWidth, height: spaceHeight } = useWindowSize(); // Space must be root element
const { graph: spaceGraph, connection: spaceConnection } = useActiveConnection(
  computed(() => spacePtr.value ?? LOCAL_SPACE_PTR),
);
const windows = spaceGraph.getChildrenRef(spacePtr, NodeType.VIEW);
const window = computed(() => windows.value[0]); // assumes at :OneRootWindow for now

const mainBox = computed(() => ({
  left: 0,
  top: BAR_HEIGHT,
  width: spaceWidth.value,
  height: spaceHeight.value - BAR_HEIGHT - BAR_OFFSET,
}));
const omnibarRef = ref<InstanceType<typeof Omnibar> | null>(null);

// sync browser title
const browserTitle = useTitle();
watch([canvas.focusedViewPtr, bench], () => {
  const benchPostfix = bench.value == null ? "Bench" : bench.value?.slug;
  let viewTitle = null;
  if (canvas.focusedViewPtr.value != null) {
    const viewAncestors = [canvas.graph.get(canvas.focusedViewPtr.value)!].concat(
      ...(canvas.graph.getAncestors(canvas.focusedViewPtr.value, [NodeType.VIEW]) as ViewData[]),
    );
    viewTitle = viewAncestors.find((ancestor) => ancestor.title != null)?.title;
  }

  browserTitle.value = viewTitle ? `${viewTitle} - ${benchPostfix}` : benchPostfix;
});
</script>
<template>
  <!-- Space -->
  <div
    ref="spaceRef"
    class="scrollbar-none max-h-screen w-full overflow-hidden overscroll-none bg-gray-100 text-sm"
    :class="[isDragging ? 'yselect-none pointer-events-none' : '']"
    :style="{ width: spaceWidth + 'px', height: spaceHeight + 'px' }"
    @contextmenu.stop.prevent="() => {} /* suppress generic context menu */"
  >
    <!-- Bar -->
    <Bar
      ref="barRef"
      class="w-full border-b-2 border-gray-300"
      :style="{ height: BAR_HEIGHT + 'px' }"
      :space-graph="spaceGraph"
      :space-connection="spaceConnection"
      :box="{ x: 0, y: 0, width: spaceWidth, height: BAR_HEIGHT }"
    />
    <!-- Window root -->
    <Split
      v-if="window"
      :type="ViewType.WINDOW"
      :self="toNodeReference(window)"
      :size="mainBox"
      :focus="window.focus"
      :name="window.name"
      :title="window.name"
      :text="window.text"
      :orientation="Orientation.HORIZONTAL"
      :style="{ marginTop: BAR_OFFSET + 'px' }"
    />
    <!-- Overlays -->
    <ToastOverlay anchor="bottom-right" :box="mainBox" />
    <Omnibar ref="omnibarRef" :box="mainBox" />
    <TooltipOverlay />
    <MenuOverlay />
  </div>
</template>
<style>
/* stop overscrolling */
* {
  overscroll-behavior: none;
  scrollbar-gutter: overlay;
}

/* hide scrollbar with .scrollbar-none */
.scrollbar-none {
  scrollbar-width: none;
  -ms-overflow-style: none;
}
.scrollbar-none::-webkit-scrollbar {
  display: none;
}

/** make selections match primary color */
::selection {
  background-color: #fcd34d;
}

.caret-transparent {
  caret-color: transparent;
}

/* marks shouldn't be ugly */
mark {
  background-color: transparent;
  color: inherit;
  font-weight: bold;
  text-decoration: underline;
  text-underline-offset: 2px;
}
</style>
@/system/client

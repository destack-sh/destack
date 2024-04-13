<script lang="ts" setup>
import { NodeType, Orientation, ViewType } from "@/proto/wire";
import { toNodeReference } from "@/proto/wiring";
import { spacePtr } from "@/system/client";
import { IconInline } from "@/system/icon";
import { getNodeIcon } from "@/system/lang";
import { bench, canvas, spaceConnection, spaceGraph } from "@/system/space";
import { toaster } from "@/system/toast";
import { _setDragImage, activeDragged } from "@/utils/drag";
import { keytrap } from "@/utils/keymap";
import { isDraggingGlobal } from "@/utils/layout";
import { Casing, toCasing } from "@/utils/string";
import Split from "@/views/containers/Split.vue";
import Bar from "@/views/private/Bar.vue";
import MenuOverlay from "@/views/private/MenuOverlay.vue";
import Omnibar from "@/views/private/Omnibar.vue";
import ToastOverlay from "@/views/private/ToastOverlay.vue";
import TooltipOverlay from "@/views/private/TooltipOverlay.vue";
import { useTitle, useWindowSize } from "@vueuse/core";
import { computed, onBeforeUnmount, ref, watch } from "vue";

const BAR_HEIGHT = 42;
const BAR_OFFSET = 0;
const spaceRef = ref<HTMLElement | null>(null);
const barRef = ref<InstanceType<typeof Bar> | null>(null);
const { width: spaceWidth, height: spaceHeight } = useWindowSize(); // Space must be root element
const windows = spaceGraph.getChildrenRef(spacePtr, NodeType.VIEW);
const window = computed(() => windows.value[0]); // assumes at :OneRootWindow for now

const mainBox = computed(() => ({
  left: 0,
  top: BAR_HEIGHT,
  width: spaceWidth.value,
  height: spaceHeight.value - BAR_HEIGHT - BAR_OFFSET,
}));
const omnibarRef = ref<InstanceType<typeof Omnibar> | null>(null);

// suppress save everywhere
const unbind = keytrap.bind(["ctrl+s", "mod+s"], () => {
  toaster.info({
    key: "space.suppressSave",
    icon: "fas fa-floppy-disk",
    title: "No need to save",
    text: "Bench synchronizes automatically.",
    debounce: true,
  });
  return true;
});
onBeforeUnmount(() => unbind()); // for hot reload

// sync browser title
const browserTitle = useTitle();
watch([canvas.focusedViewPtr, bench], () => {
  const benchPostfix = bench.value == null ? "Bench" : bench.value?.slug;
  let viewTitle = null;
  if (canvas.focusedView.value != null) {
    const viewAncestors = canvas.graph.getAncestors(canvas.focusedViewPtr.value!, {
      metatypes: [NodeType.VIEW],
      includeSelf: true,
    });
    viewTitle = viewAncestors.find((ancestor) => ancestor.title != null)?.title;
  }

  browserTitle.value = viewTitle ? `${viewTitle} | ${benchPostfix}` : benchPostfix;
});
</script>
<template>
  <!-- Space -->
  <div
    ref="spaceRef"
    class="scrollbar-none max-h-screen w-full overflow-hidden overscroll-none bg-gray-100 text-sm"
    :class="[isDraggingGlobal ? 'pointer-events-none select-none' : '']"
    :style="{ width: spaceWidth + 'px', height: spaceHeight + 'px' }"
    @contextmenu.stop.prevent="() => {} /* suppress generic context menu */"
  >
    <!-- Bar -->
    <Bar
      ref="barRef"
      class="w-full border-b border-gray-300"
      :style="{ height: BAR_HEIGHT + 'px' }"
      :space-graph="spaceGraph"
      :space-connection="spaceConnection"
      :box="{ x: 0, y: 0, width: spaceWidth, height: BAR_HEIGHT }"
    />
    <!-- Space root (:OneRootWindow) -->
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
    <!-- Loading... -->
    <div
      v-else-if="!spaceConnection.isConnected.value"
      class=""
      :style="{
        width: mainBox.width + 'px',
        height: mainBox.height + 'px',
      }"
    >
      <div class="flex h-full flex-col items-center justify-center">
        <i class="fas fa-spinner-third animate-spin text-xl text-gray-400" />
      </div>
    </div>

    <!-- Overlays -->
    <ToastOverlay anchor="bottom-right" :box="mainBox" />
    <Omnibar ref="omnibarRef" :box="mainBox" />
    <TooltipOverlay />
    <MenuOverlay />
    <!-- Drag image overlay -->
    <!-- Wrapper to ensure dragImageRef is always set -->
    <div :ref="(ref) => _setDragImage(ref as any)" class="absolute -top-[100px] left-20 py-1 pl-2">
      <div
        v-if="activeDragged"
        class="max-w-48 rounded-md border border-gray-400 bg-white px-2 py-1 text-gray-900 shadow-md shadow-gray-400"
      >
        <!-- And wrapper to offset within the image to ensure the text isn't obscured by the cursor -->
        <div v-if="activeDragged.kind == 'node'" class="flex flex-row items-center">
          <IconInline v-bind="getNodeIcon(activeDragged.nodes[0])" class="mr-1 text-gray-700" />
          <span class="truncate">
            {{
              (activeDragged.nodes[0] as any).title ??
              (activeDragged.nodes[0] as any).name ??
              toCasing(NodeType[activeDragged.node.type], Casing.CAMEL)
            }}
          </span>
        </div>
        <div v-else>
          <span class="text-gray-700">{{ toCasing(activeDragged.kind, Casing.CAMEL) }}</span>
        </div>
      </div>
    </div>
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

/* marks are bold+underline by default */
mark {
  background-color: transparent;
  color: inherit;
  font-weight: bold;
  text-decoration: underline;
  text-underline-offset: 2px;
}

.mark-bold mark {
  font-weight: bold;
}

.mark-semibold mark {
  font-weight: 600;
}

.mark-underlined mark {
  text-decoration: underline;
  text-underline-offset: 2px;
}

.mark-primary mark {
  background-color: #fcd34d;
  font-weight: normal;
  text-decoration: none;
}

.mark-secondary mark {
  background-color: #bae6fd;
  font-weight: normal;
  text-decoration: none;
}
</style>

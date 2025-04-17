<script lang="ts" setup>
import { supergraph } from "@/globals";
import { renderTextLine } from "@/language/core/text";
import { NodeType, Orientation, ViewType } from "@/proto/wire";
import { isNode, toNodeRef } from "@/proto/wiring";
import { spacePtr } from "@/system/client";
import { bench, containerPtr, inspectionPtr, pagePtr, spaceConnection, spaceGraph } from "@/system/space";
import { IS_IN_ALT_MODE } from "@/ui/command";
import { getNodeTitle } from "@/ui/icon";
import { keytrap } from "@/ui/keymap";
import { IS_DRAGGING, IS_DRAGGING_OR_SELECTING } from "@/ui/layout";
import { hasActivePopover, pushDefaultContextMenu } from "@/ui/popover";
import EmptySpace from "@/views/builtin/EmptySpace.vue";
import Inaccessible from "@/views/builtin/Inaccessible.vue";
import Omnibar from "@/views/builtin/Omnibar.vue";
import Split from "@/views/containers/Split.vue";
import DragOverlay from "@/views/overlays/DragOverlay.vue";
import LightboxOverlay from "@/views/overlays/LightboxOverlay.vue";
import PopoverOverlay from "@/views/overlays/PopoverOverlay.vue";
import ToastOverlay from "@/views/overlays/ToastOverlay.vue";
import TooltipOverlay from "@/views/overlays/TooltipOverlay.vue";
import { useTitle, useWindowSize } from "@vueuse/core";
import { computed, onBeforeUnmount, ref, watch } from "vue";

const spaceRef = ref<HTMLElement | null>(null);
const { width: spaceWidth, height: spaceHeight } = useWindowSize(); // Space must be root element
const mainBox = computed(() => ({ left: 0, top: 0, width: spaceWidth.value, height: spaceHeight.value }));
const windows = spaceGraph.getChildrenRef(spacePtr, NodeType.VIEW);
const window = computed(() => windows.value[0]); // assumes :OneRootWindow for now
const omnibarRef = ref<InstanceType<typeof Omnibar> | null>(null);

// suppress save everywhere
const unbind = keytrap.bind(["ctrl+s", "mod+s"], () => true);
onBeforeUnmount(() => unbind()); // for hot reload

// sync browser title
const inspection = supergraph.getRef(inspectionPtr);
const container = supergraph.getRef(containerPtr);
const page = supergraph.getRef(pagePtr);

const browserTitle = useTitle();
watch(
  [bench, inspection],
  () => {
    const benchPostfix = bench.value == null ? "Bench" : bench.value?.name;
    const titleParts = [];
    if (inspection.value != null) {
      titleParts.push(getNodeTitle(inspection.value));
    }
    if (container.value != null) {
      titleParts.push(getNodeTitle(container.value));
    }
    if (page.value != null) {
      titleParts.push(getNodeTitle(page.value));
    }
    titleParts.push(benchPostfix);
    browserTitle.value = titleParts.filter((t) => t != null).join(" · ");
  },
  { immediate: true },
);

// sync browser slug
watch(
  bench,
  () => {
    if (bench.value != null) {
      globalThis.history.replaceState({}, "", `/${bench.value.slug}`);
    } else {
      globalThis.history.replaceState({}, "", "/");
    }
  },
  { immediate: true },
);
</script>
<template>
  <!-- Space -->
  <div
    ref="spaceRef"
    class="select-none overflow-hidden bg-white text-sm"
    :class="[
      IS_DRAGGING_OR_SELECTING || hasActivePopover ? 'select-none' : '',
      IS_DRAGGING ? 'pointer-events-none' : '',
      IS_IN_ALT_MODE ? 'altmode' : '',
    ]"
    :style="{ width: spaceWidth + 'px', height: spaceHeight + 'px' }"
    @contextmenu.stop.prevent="(e) => pushDefaultContextMenu(e)"
  >
    <!-- Space root (:OneRootWindow) -->
    <Split
      v-if="window"
      id="window"
      class=""
      :type="ViewType.WINDOW"
      :self="toNodeRef(window)"
      :focus-ptr="window.focusPtr"
      :name="window.name"
      :size="mainBox"
      :orientation="Orientation.HORIZONTAL"
      is-root
    />
    <!-- Loading... -->
    <div v-else-if="!spaceConnection.isConnected.value" class="absolute h-full w-full bg-white">
      <Inaccessible class="h-full w-full" :node="spacePtr" :connection="spaceConnection" />
    </div>
    <!-- Does not have a space (not signed, space empty or disappeared) -->
    <EmptySpace v-else class="absolute flex h-full w-full flex-col justify-center bg-white text-center" />

    <!-- Overlays -->
    <DragOverlay />
    <ToastOverlay anchor="bottom-right" :box="mainBox" />
    <Omnibar ref="omnibarRef" :box="mainBox" />
    <LightboxOverlay :box="mainBox" />
    <PopoverOverlay />
    <TooltipOverlay />
  </div>
</template>

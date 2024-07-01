<script lang="ts" setup>
import { Anchor, NodeType, Orientation, ViewType } from "@/proto/wire";
import { toNodeReference } from "@/proto/wiring";
import { spacePtr } from "@/system/client";
import { bench, canvas, space, spaceConnection, spaceGraph } from "@/system/space";
import { toaster } from "@/system/toast";
import { keytrap } from "@/utils/keymap";
import { isDraggingGlobal } from "@/utils/layout";
import { hasActivePopover } from "@/utils/menu";
import Bar from "@/views/builtins/Bar.vue";
import DragOverlay from "@/views/builtins/DragOverlay.vue";
import PopoverOverlay from "@/views/builtins/PopoverOverlay.vue";
import Omnibar from "@/views/builtins/Omnibar.vue";
import ToastOverlay from "@/views/builtins/ToastOverlay.vue";
import TooltipOverlay from "@/views/builtins/TooltipOverlay.vue";
import Split from "@/views/containers/Split.vue";
import { IS_IN_ALT_MODE, fireActionById } from "@/system/action";
import { useTitle, useWindowSize } from "@vueuse/core";
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { makeIcon } from "@/system/icon";
import { assignSpaceInPackage } from "@/system/space";
import Button from "@/views/controls/Button.vue";
import { DEFAULT_BAR_POSITION, DEFAULT_HEADER_HEIGHT, createDesktopProSpace } from "@/views/canvas";
import { user } from "@/system/user";

const BAR_WIDTH = DEFAULT_HEADER_HEIGHT;
const BAR_HEIGHT = DEFAULT_HEADER_HEIGHT;
const TOP_INSET_WITHOUT_BAR = 1;

const spaceRef = ref<HTMLElement | null>(null);
const barRef = ref<InstanceType<typeof Bar> | null>(null);
const { width: spaceWidth, height: spaceHeight } = useWindowSize(); // Space must be root element
const windows = spaceGraph.getChildrenRef(spacePtr, NodeType.VIEW);
const window = computed(() => windows.value[0]); // assumes at :OneRootWindow for now

const barPosition = computed(() => space.value?.barPosition ?? DEFAULT_BAR_POSITION);
const barOrientation = computed(() =>
  barPosition.value == Anchor.TOP || barPosition.value == Anchor.BOTTOM ? Orientation.HORIZONTAL : Orientation.VERTICAL,
);
const barOffset = computed(() => {
  if (barPosition.value == Anchor.LEFT) return { left: 0, top: TOP_INSET_WITHOUT_BAR };
  else if (barPosition.value == Anchor.TOP) return { left: 0, top: 0 };
  else if (barPosition.value == Anchor.RIGHT) return { left: spaceWidth.value - BAR_WIDTH, top: TOP_INSET_WITHOUT_BAR };
  else if (barPosition.value == Anchor.BOTTOM)
    return { left: 0, top: TOP_INSET_WITHOUT_BAR + spaceHeight.value - BAR_HEIGHT };
  else throw new Error(`unexpected bar position: ${barPosition.value}`);
});
const mainOffset = computed(() => {
  if (barPosition.value == Anchor.LEFT) return { left: BAR_WIDTH, top: TOP_INSET_WITHOUT_BAR };
  else if (barPosition.value == Anchor.TOP) return { left: 0, top: BAR_HEIGHT };
  else if (barPosition.value == Anchor.RIGHT) return { left: 0, top: TOP_INSET_WITHOUT_BAR };
  else if (barPosition.value == Anchor.BOTTOM) return { left: 0, top: TOP_INSET_WITHOUT_BAR };
  else throw new Error(`unexpected bar position: ${barPosition.value}`);
});
const mainBoxStyle = computed(() => ({
  left: mainOffset.value.left + "px",
  top: mainOffset.value.top + "px",
}));
const mainBox = computed(() => ({
  ...mainOffset.value,
  width: spaceWidth.value - (barPosition.value == Anchor.LEFT || barPosition.value == Anchor.RIGHT ? BAR_WIDTH : 0),
  height:
    spaceHeight.value -
    (barPosition.value == Anchor.TOP || barPosition.value == Anchor.BOTTOM ? BAR_HEIGHT : 0) -
    (barPosition.value == Anchor.TOP ? 0 : TOP_INSET_WITHOUT_BAR),
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
  if (canvas.focusedViewPtr.value != null) {
    const viewAncestors = canvas.graph.getAncestors(canvas.focusedViewPtr.value, {
      metatypes: [NodeType.VIEW],
      includeSelf: true,
    });
    viewTitle = viewAncestors.find((ancestor) => ancestor.name != null || ancestor.title != null)?.title;
  }

  browserTitle.value = viewTitle ? `${viewTitle} | ${benchPostfix}` : benchPostfix;
});
</script>
<template>
  <!-- Space -->
  <div
    ref="spaceRef"
    class="scrollbar-none h-full max-h-screen w-full overflow-hidden overscroll-none bg-gray-100 text-sm"
    :class="[
      isDraggingGlobal || hasActivePopover ? 'pointer-events-none select-none' : '',
      IS_IN_ALT_MODE ? 'altmode' : '',
    ]"
    :style="{ width: spaceWidth + 'px', height: spaceHeight + 'px' }"
    @contextmenu.stop.prevent="() => {} /* suppress generic context menu */"
  >
    <!-- Top inset if we don't have a bar -->
    <!-- (to add some spacing so that the space contents don't look 'squished' to the top) -->
    <div
      v-if="barPosition != Anchor.TOP"
      class="absolute top-0 w-full bg-gray-100"
      :style="{ height: TOP_INSET_WITHOUT_BAR + 'px' }"
    />
    <!-- Bar -->
    <Bar
      ref="barRef"
      class="absolute border-gray-200 bg-gray-100"
      :class="barOrientation == Orientation.VERTICAL ? 'border-x' : 'border-y'"
      :anchor="barPosition"
      :orientation="barOrientation"
      :space-graph="spaceGraph"
      :space-connection="spaceConnection"
      :style="{
        left: barOffset.left + 'px',
        top: barOffset.top + 'px',
        width: (barOrientation == Orientation.HORIZONTAL ? spaceWidth : BAR_WIDTH) + 'px',
        height: (barOrientation == Orientation.HORIZONTAL ? BAR_HEIGHT : spaceHeight) + 'px',
      }"
    />
    <!-- Space root (:OneRootWindow) -->
    <Split
      v-if="window"
      class="absolute"
      :style="mainBoxStyle"
      :type="ViewType.WINDOW"
      :self="toNodeReference(window)"
      :size="mainBox"
      :focus="window.focus"
      :name="window.name"
      :title="window.name"
      :text="window.text"
      :orientation="Orientation.HORIZONTAL"
    />
    <!-- Loading... -->
    <div
      v-else-if="!spaceConnection.isConnected.value"
      class="absolute bg-white"
      :style="{ width: mainBox.width + 'px', height: mainBox.height + 'px', ...mainBoxStyle }"
    >
      <div class="flex h-full flex-col items-center justify-center">
        <Transition
          enter-from-class="opacity-0"
          enter-active-class="transition-opacity duration-200"
          enter-to-class="opacity-100"
          appear
        >
          <i class="fas fa-spinner-third animate-spin text-xl text-gray-400" />
        </Transition>
      </div>
    </div>
    <!-- Does not have a space (not signed, space empty or disappeared) -->
    <div
      v-else
      class="absolute flex flex-col justify-center bg-white text-center"
      :style="{ width: mainBox.width + 'px', height: mainBox.height + 'px', ...mainBoxStyle }"
    >
      <div v-if="space && bench" class="flex w-fit flex-col gap-y-2 self-center">
        <!-- Space empty for some reason -->
        <span>
          <i class="fas fa-empty-set mr-1.5 text-gray-500" />
          <span class="text-gray-600">Space Is Empty</span>
        </span>
        <Button
          name="Create"
          :icon="makeIcon('fas fa-redo-alt')"
          title="Restore Default"
          @click="() => createDesktopProSpace(canvas.tx(), space!)"
        />
      </div>
      <div v-else-if="bench" class="flex w-fit flex-col gap-y-2 self-center">
        <!-- Space inaccessible for some reason -->
        <span>
          <i class="fas fa-exclamation-triangle mr-1.5 text-gray-500" />
          <span class="text-gray-600">Space Not Found</span>
        </span>
        <Button
          name="Create"
          :icon="makeIcon('fas fa-plus')"
          title="Create Space"
          @click="() => assignSpaceInPackage()"
        />
      </div>
      <div v-else-if="user" class="flex flex-col gap-y-2 self-center">
        <!-- Logged in, but not on any space (not sure if this should even show or just auto-redirect?) -->
        <span
          >^
          <i class="fas fa-exclamation-triangle mr-1.5 text-gray-500" />
          <span class="text-gray-600">You're Lost in Space</span>
        </span>
        <Button
          name="GoHome"
          :icon="makeIcon('fas fa-home')"
          title="Go Home"
          @click="fireActionById('user.misc.goToHome')"
        />
      </div>
      <div v-else class="flex flex-col gap-y-2 self-center">
        <!-- Not logged in, not on a space (general landing page should go here) -->
        <h2 class="mb-1.5 text-2xl font-bold">Bench</h2>
        <Button
          name="LogIn"
          :icon="makeIcon('fas fa-arrow-right-to-bracket')"
          title="Log In"
          @click="fireActionById('user.auth.login')"
        />
      </div>
    </div>

    <!-- Overlays -->
    <ToastOverlay anchor="bottom-right" :box="mainBox" />
    <Omnibar ref="omnibarRef" :box="mainBox" />
    <TooltipOverlay />
    <PopoverOverlay />
    <DragOverlay />
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

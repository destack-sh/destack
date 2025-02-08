<script lang="ts" setup>
import { NodeType, Orientation, ViewType } from "@/proto/wire";
import { toNodeRef } from "@/proto/wiring";
import { spacePtr } from "@/system/client";
import { supergraph } from "@/globals";
import { bench, inspectionPtr, spaceConnection, spaceGraph } from "@/system/space";
import { IS_IN_ALT_MODE } from "@/ui/action";
import { keytrap } from "@/ui/keymap";
import { isDraggingGlobal } from "@/ui/layout";
import { hasActivePopover, pushDefaultContextMenu } from "@/ui/popover";
import DragOverlay from "@/views/builtins/DragOverlay.vue";
import EmptySpace from "@/views/builtins/EmptySpace.vue";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import Omnibar from "@/views/builtins/Omnibar.vue";
import PopoverOverlay from "@/views/builtins/PopoverOverlay.vue";
import ToastOverlay from "@/views/builtins/ToastOverlay.vue";
import TooltipOverlay from "@/views/builtins/TooltipOverlay.vue";
import Split from "@/views/containers/Split.vue";
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
const inspectedNode = supergraph.getRef(inspectionPtr);
const browserTitle = useTitle();
watch(
  [bench, inspectedNode],
  () => {
    const benchPostfix = bench.value == null ? "Bench" : bench.value?.slug;
    const nodeTitle =
      (inspectedNode.value as any)?.slug ?? (inspectedNode.value as any)?.name ?? (inspectedNode.value as any)?.title;
    browserTitle.value = nodeTitle ? `${nodeTitle} | @${benchPostfix}` : `@${benchPostfix}`;
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
      isDraggingGlobal || hasActivePopover ? 'pointer-events-none select-none' : '',
      IS_IN_ALT_MODE ? 'altmode' : '',
    ]"
    :style="{ width: spaceWidth + 'px', height: spaceHeight + 'px' }"
    @contextmenu.stop.prevent="(e) => pushDefaultContextMenu(e)"
  >
    <!-- Space root (:OneRootWindow) -->
    <Split
      v-if="window"
      id="window"
      class="absolute"
      :type="ViewType.WINDOW"
      :self="toNodeRef(window)"
      :focus="window.focus"
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
    <TooltipOverlay />
    <PopoverOverlay />
  </div>
</template>
<style>
/*
 * General styles
 */

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
<style>
/*
 * ProseMirror styling
 */

@import url("/node_modules/prosemirror-view/style/prosemirror.css");

.pm-text {
  @apply text-gray-900;
  line-height: 1.65;
}
.pm-text strong {
  @apply font-semibold;
}
.pm-text .line:first-child {
  @apply mt-0; /* ignore top margin */
}
.pm-text .line:last-child {
  @apply mb-0; /* ignore bottom margin */
}
.pm-text .line {
  @apply min-h-[30px] py-[3px] text-base;
}
.pm-text hr {
  @apply my-2 border-gray-200 p-0 focus:outline-none focus:ring-0;
}
.pm-text h1 {
  @apply mb-2 mt-4 text-2xl font-bold;
  line-height: 1.2;
}
.pm-text h2 {
  @apply mb-1.5 mt-2.5 text-xl font-bold;
  line-height: 1.4;
}
.pm-text h3 {
  @apply mb-0.5 mt-1.5 text-lg font-bold;
  line-height: 1.5;
}
.pm-text h4 {
  @apply mb-0.5 mt-1.5 text-base font-medium;
  line-height: 1.5;
}
.pm-text code {
  @apply rounded bg-gray-100 px-0.5;
}
.pm-text blockquote {
  @apply my-2 border-l-4 border-gray-400 py-[1px] pl-2;
}
.pm-text div.callout {
  @apply my-2 rounded bg-gray-100 px-2 py-2.5;
}
/* TODO: UI: callout/heading/etc. line icons should be editable */
.pm-text div.callout::before {
  content: "\f06a"; /* fa-icon: circle-exclamation */
  font-family: "Font Awesome 6 Pro";
  font-weight: 900;
  @apply mr-1 px-1 text-gray-700;
}

/* Span Nodes */
.pm-text span.spanNode {
  @apply rounded px-1 py-0;
}
.pm-text span.spanNode:hover {
  @apply bg-gray-100;
}
.pm-text span.spanNode .icon {
  @apply mr-1.5 text-gray-700;
}
.pm-text span.spanNode:hover .icon {
  @apply text-gray-700;
}
.pm-text span.spanNode .name {
  @apply underline decoration-gray-300 underline-offset-3;
}
.pm-text span.pm-textMirror-selectednode.spanNode .name {
  @apply bg-primary-100 text-primary-700 decoration-primary-700;
}
.altmode .pm-text span.spanNode:hover,
.pm-text span.spanNode[data-node-type="file"]:hover .name {
  @apply cursor-pointer;
}
.altmode .pm-text span.spanNode:hover .name,
.pm-text span.spanNode[data-node-type="file"]:hover .name {
  @apply decoration-primary-700;
}

.ProseMirror-focused {
  outline: none;
}
.ProseMirror-selectednode {
  @apply p-2 outline-primary-700;
}
.ProseMirror[contenteditable="false"] {
  user-select: text; /* let user highlight text */
  -webkit-user-select: text;
  -moz-user-select: text;
}
</style>

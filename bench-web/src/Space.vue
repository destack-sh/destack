<script lang="ts" setup>
import { supergraph } from "@/globals";
import { renderTextLine } from "@/language/core/text";
import { NodeType, Orientation, ViewType } from "@/proto/wire";
import { isNode, toNodeRef } from "@/proto/wiring";
import { spacePtr } from "@/system/client";
import { bench, inspectionPtr, spaceConnection, spaceGraph } from "@/system/space";
import { IS_IN_ALT_MODE } from "@/ui/command";
import { getNodeIcon } from "@/ui/icon";
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
import RunOverlay from "@/views/overlays/RunOverlay.vue";
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
const inspectedNode = supergraph.getRef(inspectionPtr);
const inspectedNodeTitle = computed(() => {
  const node = inspectedNode.value;
  if (isNode(node, NodeType.CHANNEL)) {
    return `#${node.name}`;
  } else if (isNode(node, NodeType.VIEW)) {
    return node.title ?? node.name;
  } else if ((node as any)?.title != null) {
    return renderTextLine((node as any).title);
  } else {
    return (node as any)?.slug ?? (node as any)?.name;
  }
});
const inspectedNodeIcon = computed(() => {
  const node = inspectedNode.value;
  return node != null ? getNodeIcon(node) : undefined;
});
const browserTitle = useTitle();
watch(
  [bench, inspectedNode],
  () => {
    const benchPostfix = bench.value == null ? "Bench" : bench.value?.name;
    const nodeTitle = inspectedNodeTitle.value;
    browserTitle.value = nodeTitle ? `${nodeTitle} · ${benchPostfix}` : `${benchPostfix}`;
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
    <RunOverlay />
    <DragOverlay />
    <ToastOverlay anchor="bottom-right" :box="mainBox" />
    <Omnibar ref="omnibarRef" :box="mainBox" />
    <LightboxOverlay :box="mainBox" />
    <PopoverOverlay />
    <TooltipOverlay />
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

/** Basics */
.pm-text {
  @apply text-gray-900;
}
.pm-text strong {
  @apply font-semibold;
}
.pm-text code {
  @apply bg-gray-100 px-1 py-0.5 font-mono text-sm;
}
/* NOTE :Cleanup: we select only direct line descendants to avoid overriding nested pm-text instances that want to inherit
   (like for Titles we sometimes inherit from the containing line, so we don't want to override that) */
.pm-text.pm-base > * > .line,
.pm-text.pm-base > * > * > .line,
.pm-text.pm-base > * > .line-block.page,
.pm-text.pm-base > * > * > .line-block.page {
  @apply text-base;
  line-height: 1.65;
}
.pm-text.pm-sm > * > .line,
.pm-text.pm-sm > * > * > .line,
.pm-text.pm-sm > * > .line-block.page,
.pm-text.pm-sm > * > * > .line-block.page {
  @apply text-sm;
  line-height: 1.65;
}
.pm-text.pm-inactive .line,
.pm-text.pm-inactive .line-block {
  @apply select-none;
}
.pm-text.pm-inherit .line,
.pm-text.pm-inherit .line-block.page {
  @apply text-inherit;
}
.pm-text .line,
.pm-text .line-block.page {
  @apply relative px-0.5 py-[3px] transition-colors duration-150;
}
.pm-text .line,
.pm-text .line-block {
  @apply transition-colors duration-150;
}
.pm-text.pm-compact .line,
.pm-text.pm-compact .line-block.page {
  @apply px-0 py-0;
}
.pm-text.pm-compact.pm-paddingless .line,
.pm-text.pm-compact.pm-paddingless .line-block.page {
  @apply mb-0 mt-0 px-0 py-0;
}
.pm-text.pm-truncate .line,
.pm-text.pm-truncate .line-block {
  @apply truncate;
}

/** Placeholders */
.pm-text .placeholder {
  @apply pointer-events-none select-none text-gray-400;
}
.pm-text .ProseMirror-focused .placeholder-hidden {
  @apply opacity-100;
}
.pm-text .placeholder-hidden {
  @apply opacity-0;
}

/** Links */
.pm-text a {
  @apply cursor-pointer text-gray-400 underline decoration-gray-300 underline-offset-2 transition-colors duration-150;
}
.pm-text a:hover {
  @apply text-primary-700 decoration-primary-700;
}

/** Citations */
.pm-text a.citation {
  @apply rounded-full bg-gray-100 px-1 py-0.5 no-underline;
}
.pm-text a.citation:hover {
  @apply bg-primary-100 text-primary-700;
}

/** Selected */
.pm-text .line.selected,
.pm-text .line-block.selected {
  @apply bg-orange-400/20;
}
.pm-text hr.selected {
  @apply border-orange-300;
}
.pm-text code.line.selected {
  @apply bg-orange-400/20;
}
.pm-text p.line.callout.selected {
  @apply bg-orange-400/20;
}
.pm-text .line-block.page.selected {
  @apply bg-orange-400/20;
}

/** Dragging */
.pm-text .line.dragging,
.pm-text .line-block.dragging {
  @apply opacity-50;
}

/** Other lines */
.pm-text blockquote.line,
.pm-text.pm-compact blockquote.line {
  @apply my-2 border-l-4 border-gray-400 pl-2;
}
.pm-text p.line.callout,
.pm-text.pm-compact p.line.callout {
  @apply my-2 rounded bg-gray-100 px-2 py-3;
}
.pm-text p.line.callout::before {
  content: "\f06a"; /* fa-icon: circle-exclamation */
  font-family: "Font Awesome 6 Pro";
  font-weight: 900;
  @apply mr-1 pl-1.5 pr-2 text-gray-700;
}
.pm-text hr {
  @apply my-3 border-gray-200 focus:outline-none focus:ring-0;
}
.pm-text.pm-base > * > code.line {
  @apply my-2.5 block w-full rounded border border-gray-200 bg-gray-100 px-3 py-3 text-sm;
}
.pm-text.pm-compact > * > code.line {
  @apply my-1.5 rounded border border-gray-200 px-3 py-2.5;
}
.pm-text .hljs {
  @apply bg-transparent;
}

/** Line handles */
.pm-text .line-handle {
  @apply absolute -left-[6px] top-[3px] z-40 flex -translate-x-full text-base text-gray-400 opacity-0 transition-colors duration-150;
}
.pm-text .line-handle-button {
  @apply w-5 rounded text-base transition-colors duration-150;
}
.pm-text .line-handle-button:hover {
  @apply bg-gray-100 text-gray-700;
}
.pm-text .line-block > .line-handle {
  @apply mt-[7px];
}
.pm-text .line-block.page > .line-handle,
.pm-text .line-block.file > .line-handle,
.pm-text .line-block.task > .line-handle {
  @apply mt-0;
}
.pm-text .line-block:hover .line-handle,
.pm-text .line:hover .line-handle,
.pm-text .line-block:focus-within .line-handle,
.pm-text .line:focus-within .line-handle {
  @apply opacity-100;
}

/* Block Lines */
.pm-text .line-block {
  @apply relative my-1 cursor-default py-1;
}
.pm-text .line-block.page {
  @apply my-0;
}
.pm-text .line-block.page:hover:not(.selected),
.pm-text .line-block.page:focus-within {
  @apply cursor-pointer bg-gray-100;
}
.pm-text .line-block.ProseMirror-selectednode {
  @apply p-0 outline-none;
}

/* Headings */
.pm-text:not(.pm-inherit) h1.line {
  @apply text-3xl font-bold;
  line-height: 1.2;
}
.pm-text:not(.pm-paddingless) h1.line {
  @apply mb-2 mt-4;
}
.pm-text.pm-compact:not(.pm-inherit):not(.pm-paddingless) h1.line {
  @apply mb-1 mt-2;
}
.pm-text:not(.pm-inherit) h2.line {
  @apply text-2xl font-bold;
  line-height: 1.4;
}
.pm-text:not(.pm-paddingless) h2.line {
  @apply mb-1.5 mt-2.5;
}
.pm-text.pm-compact:not(.pm-inherit):not(.pm-paddingless) h2.line {
  @apply mb-0.5 mt-1.5;
}
.pm-text:not(.pm-inherit) h3.line {
  @apply text-xl font-bold;
  line-height: 1.5;
}
.pm-text:not(.pm-paddingless) h3.line {
  @apply mb-0.5 mt-1.5;
}
.pm-text.pm-compact:not(.pm-inherit):not(.pm-paddingless) h3.line {
  @apply mb-0 mt-1;
}
.pm-text:not(.pm-inherit) h4.line {
  @apply text-lg font-bold;
  line-height: 1.5;
}
.pm-text:not(.pm-paddingless) h4.line {
  @apply mb-0.5 mt-1.5;
}
.pm-text.pm-compact:not(.pm-inherit):not(.pm-paddingless) h4.line {
  @apply mb-0 mt-0.5;
}

/* Lists */
.pm-text ul li.line.list-unordered {
  @apply list-inside list-disc pl-1;
}
.pm-text ol li.line.list-ordered {
  @apply list-inside list-decimal pl-1;
  list-style-position: inside;
  list-style-type: decimal;
}
.pm-text ol ol li.line.list-ordered {
  list-style-type: lower-alpha;
}
.pm-text ol ol ol li.line.list-ordered {
  list-style-type: lower-roman;
}

/* Span Nodes */
.pm-text .span-node-view {
  @apply rounded px-1 transition-colors duration-150;
}
.pm-text .span-node-view:hover,
.pm-text .span-node-view:focus-within {
  @apply cursor-pointer bg-gray-100;
}

.ProseMirror-focused {
  outline: none;
}
.ProseMirror-selectednode {
  @apply outline-primary-700;
}
.ProseMirror[contenteditable="false"] {
  user-select: text; /* let user highlight text */
  -webkit-user-select: text;
  -moz-user-select: text;
}
</style>

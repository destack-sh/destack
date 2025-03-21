<script lang="ts" setup>
import { NodeType, Orientation, RectangleData, ViewData, ViewType } from "@/proto/wire";
import { isNode, type TypedNodeReferenceData } from "@/proto/wiring";
import { canvas, spaceGraph } from "@/system/space";
import type { CommandMapKit } from "@/ui/command";
import type { SplitLayout } from "@/ui/layout";
import { DEFAULT_ORIENTATION, MIN_SPLIT_SIZE, useSplitView } from "@/ui/layout";
import Empty from "@/views/builtins/Empty.vue";
import type { ViewExpose } from "@/views/common";
import { type ViewEmits } from "@/views/common";
import { getViewBinding, getViewComponent } from "@/views/registry";
import { computed, ref, toRef, type Ref } from "vue";

const props = defineProps<
  {
    self: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    size: Required<Pick<RectangleData, "width" | "height">>;
    isRoot?: boolean;
  } & Pick<ViewData, "type" | "name" | "title" | "icon" | "orientation" | "focus">
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");

const splits = spaceGraph.getChildrenRef(self, NodeType.VIEW, { ignoreAncestors: true });

const focusedSplitIdx: Ref<number | null> = computed(() => {
  if (splits.value.length == 0) return null;
  if (props.focus?.nodesPtr.length ?? 0 > 0) {
    const focusedId = props.focus!.nodesPtr[0].id;
    const focusedSplitIdx = splits.value.findIndex((split) => split.id == focusedId);
    return focusedSplitIdx >= 0 ? focusedSplitIdx : 0;
  } else {
    return 0;
  }
});
const orientation = computed(() => props.orientation ?? DEFAULT_ORIENTATION);
const isWindow = computed(() => props.type == ViewType.WINDOW);
const isHorizontal = computed(() => orientation.value == Orientation.HORIZONTAL);

const BORDER_SIZE = 1;
const DRAGGABLE_SIZE = 4;
const splitLayout: Ref<SplitLayout> = computed(() => ({
  orientation: orientation.value,
  minPx: MIN_SPLIT_SIZE,
  dividerSize: BORDER_SIZE,
}));
const containerRef: Ref<HTMLElement | null> = ref(null);
// TODO :Cleanup: use view state instead of canvas.tx in useSplitView
const { sizedViews, draggingIdx } = useSplitView(splits, toRef(props, "size"), containerRef, splitLayout, canvas.tx);

// commands
const commands: Partial<CommandMapKit<"view">> = {
  // navigate
  "view.navigate.closeFrame": {
    command: (command, ctx) => {
      const split = ctx.nodes?.[0];
      if (!isNode(split, NodeType.VIEW)) return false;
      canvas.removeView(spaceGraph, split);
    },
  },
  "view.navigate.focusPreviousFrame": {
    isEnabled: isWindow,
    command: (command, ctx) => {
      const allFrames = canvas.frames;
      const split = ctx.nodes?.[0];
      if (!isNode(split, NodeType.VIEW)) return false;
      const currentIdx = allFrames.findIndex((v) => v.id == split.id);
      const prevIdx = ((currentIdx ?? 0) - 1 + allFrames.length) % allFrames.length;
      canvas.focus({ node: allFrames[prevIdx] });
    },
  },
  "view.navigate.focusNextFrame": {
    isEnabled: isWindow,
    command: (command, ctx) => {
      const allFrames = canvas.frames;
      const split = ctx.nodes?.[0];
      if (!isNode(split, NodeType.VIEW)) return false;
      const currentIdx = allFrames.findIndex((v) => v.id == split.id);
      const nextIdx = ((currentIdx ?? 0) + 1) % canvas.frames.length;
      canvas.focus({ node: allFrames[nextIdx] });
    },
  },
  "view.navigate.closeSplit": {
    command: (command, ctx) => {
      const split = ctx.nodes?.[0];
      if (!isNode(split, NodeType.VIEW)) return false;
      canvas.removeView(spaceGraph, split);
    },
  },
  "view.navigate.focusNextSplit": {
    command: (command, ctx) => {
      const split = ctx.nodes?.[0];
      if (!isNode(split, NodeType.VIEW)) return false;
      const splitIdx = splits.value.findIndex((v) => v.id == split.id);
      const nextIdx = (splitIdx + 1) % splits.value.length;
      canvas.focus({ node: splits.value[nextIdx] });
    },
  },
  "view.navigate.focusPreviousSplit": {
    command: (command, ctx) => {
      const split = ctx.nodes?.[0];
      if (!isNode(split, NodeType.VIEW)) return false;
      const splitIdx = splits.value.findIndex((v) => v.id == split.id);
      const prevIdx = (splitIdx - 1 + splits.value.length) % splits.value.length;
      canvas.focus({ node: splits.value[prevIdx] });
    },
  },
};

canvas.registerView(self, id);
defineExpose<ViewExpose>({ self, commands});
</script>
<template>
  <div
    ref="containerRef"
    class="relative"
    :style="{ width: size.width + 'px', height: size.height + 'px' }"
    :class="[draggingIdx != null ? (isHorizontal ? 'cursor-ew-resize' : 'cursor-ns-resize') : '']"
  >
    <!-- Frames -->
    <template v-for="({ left, top, width, height, view }, viewIdx) in sizedViews" :key="view.id">
      <!-- Frame -->
      <div
        class="absolute border-gray-200"
        :style="{
          borderLeftWidth: viewIdx > 0 && isHorizontal ? BORDER_SIZE + 'px' : '0',
          borderTopWidth: viewIdx > 0 && !isHorizontal ? BORDER_SIZE + 'px' : '0',
          left: left + 'px',
          top: top + 'px',
          width: width + 'px',
          height: height + 'px',
        }"
        data-root-element="true"
      >
        <!-- Frame content -->
        <component
          :is="getViewComponent(view.type)"
          v-if="getViewComponent(view.type) != null"
          :is-root="isRoot"
          v-bind="
            getViewBinding(view, {
              width: isHorizontal && viewIdx > 0 ? width - BORDER_SIZE : width,
              height: !isHorizontal && viewIdx > 0 ? height - BORDER_SIZE : height,
            })
          "
        />
        <div v-else class="bg-red-100 text-center">{{ view.type }}</div>
      </div>
      <!-- Frame divider (draggable) -->
      <div
        v-if="viewIdx > 0"
        class="pointer-events-auto absolute transition-colors duration-200"
        :class="[
          isHorizontal ? 'cursor-ew-resize' : 'cursor-ns-resize',
          draggingIdx == viewIdx - 1 ? 'bg-gray-400' : 'bg-transparent hover:bg-gray-300',
        ]"
        :style="
          isHorizontal
            ? {
                width: DRAGGABLE_SIZE + 'px',
                left: left - DRAGGABLE_SIZE / 2 + 'px',
                top: top + 'px',
                height: height + 'px',
              }
            : {
                height: DRAGGABLE_SIZE + 'px',
                left: left + 'px',
                top: top - DRAGGABLE_SIZE / 2 + 'px',
                width: width + 'px',
              }
        "
        data-outside-view="true"
        @mousedown="draggingIdx = viewIdx - 1"
      />
    </template>
    <!-- No frames -->
    <Empty
      v-if="sizedViews.length === 0"
      data-contextmenu-items="view.navigate*close*frame*,view.layout*"
      :type="type"
      class="flex h-full w-full flex-col items-center justify-center"
    />
  </div>
</template>

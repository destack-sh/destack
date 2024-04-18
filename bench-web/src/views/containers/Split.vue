<script lang="ts" setup>
import { BoxData, NodeReferenceData, NodeType, Orientation, ViewData, ViewType } from "@/proto/wire";
import type { ActionContext, ActionMapImplementation } from "@/system/action";
import { useExistingConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { DEFAULT_ORIENTATION, MIN_SPLIT_SIZE, useSplitView } from "@/utils/layout";
import type { SplitLayout } from "@/utils/layout";
import { getViewBinding, getViewComponent } from "@/views";
import { viewEmits } from "@/views/common";
import type { ViewExposed } from "@/views/common";
import { computed, ref, toRef, type Ref } from "vue";
import Empty from "@/views/builtins/Empty.vue";
import type { TypedNodeReferenceData } from "@/proto/wiring";

const props = defineProps<
  {
    self: TypedNodeReferenceData<NodeType.VIEW>;
    size: Required<Pick<BoxData, "width" | "height">>;
  } & Pick<ViewData, "type" | "name" | "title" | "text" | "icon" | "orientation" | "focus">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
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
const hasFocusedSplit = computed(() => focusedSplitIdx.value != null);

const BORDER_SIZE = 1;
const DRAGGABLE_SIZE = 4;
const splitLayout: Ref<SplitLayout> = computed(() => ({
  orientation: orientation.value,
  minPx: MIN_SPLIT_SIZE,
  dividerSize: BORDER_SIZE,
}));
const containerRef: Ref<HTMLElement | null> = ref(null);
const { sizedViews, draggingIdx } = useSplitView(
  splits,
  toRef(props, "size"),
  containerRef,
  splitLayout,
  spaceConnection,
);

// actions
const getSplitFromContext = (ctx: ActionContext | undefined) => {
  const matchingSplit = splits.value.find((split) => split.id == ctx?.triggerNode?.id);
  return matchingSplit ?? splits.value[focusedSplitIdx.value!];
};
const actions: Partial<ActionMapImplementation<"view">> = {
  "view.navigate.closeFrame": {
    isEnabled: computed(() => hasFocusedSplit.value && isWindow.value),
    action: (action, ctx) => {
      const focusedSplit = getSplitFromContext(ctx);
      if (focusedSplit == null) return false;
      canvas.removeView(spaceConnection.tx, spaceGraph, focusedSplit);
    },
  },
  "view.navigate.focusPreviousFrame": {
    isEnabled: isWindow,
    action: (action, ctx) => {
      const allFrames = canvas.frames;
      const focusedSplit = getSplitFromContext(ctx);
      if (focusedSplit == null) return false;
      const currentIdx = allFrames.findIndex((window) => window.id == focusedSplit.id);
      const prevIdx = ((currentIdx ?? 0) - 1 + allFrames.length) % allFrames.length;
      canvas.focus(spaceConnection.tx, { node: allFrames[prevIdx] });
    },
  },
  "view.navigate.focusNextFrame": {
    isEnabled: isWindow,
    action: (action, ctx) => {
      const allFrames = canvas.frames;
      const focusedSplit = getSplitFromContext(ctx);
      if (focusedSplit == null) return false;
      const currentIdx = allFrames.findIndex((window) => window.id == focusedSplit.id);
      const nextIdx = ((currentIdx ?? 0) + 1) % canvas.frames.length;
      canvas.focus(spaceConnection.tx, { node: allFrames[nextIdx] });
    },
  },
  "view.navigate.closeSplit": {
    isEnabled: computed(() => hasFocusedSplit.value && !isWindow.value),
    action: (action, ctx) => {
      const focusedSplit = getSplitFromContext(ctx);
      if (focusedSplit == null) return false;
      canvas.removeView(spaceConnection.tx, spaceGraph, focusedSplit);
    },
  },
  "view.navigate.focusNextSplit": {
    isEnabled: hasFocusedSplit,
    action: (action, ctx) => {
      const focusedSplit = getSplitFromContext(ctx);
      const focusedSplitIdx = splits.value.findIndex((split) => split.id == focusedSplit.id);
      const nextIdx = (focusedSplitIdx + 1) % splits.value.length;
      canvas.focus(spaceConnection.tx, { node: splits.value[nextIdx] });
    },
  },
  "view.navigate.focusPreviousSplit": {
    isEnabled: hasFocusedSplit,
    action: (action, ctx) => {
      const focusedSplit = getSplitFromContext(ctx);
      const focusedSplitIdx = splits.value.findIndex((split) => split.id == focusedSplit.id);
      const prevIdx = (focusedSplitIdx - 1 + splits.value.length) % splits.value.length;
      canvas.focus(spaceConnection.tx, { node: splits.value[prevIdx] });
    },
  },
};

canvas.registerView(self);
defineExpose<ViewExposed>({ self, actions });
</script>
<template>
  <!-- Container -->
  <div
    ref="containerRef"
    class="relative bg-gray-100"
    :style="{ width: size.width + 'px', height: size.height + 'px' }"
    :class="[draggingIdx != null ? (isHorizontal ? 'cursor-ew-resize' : 'cursor-ns-resize') : '']"
  >
    <!-- Frames -->
    <template v-for="({ left, top, width, height, view }, viewIdx) in sizedViews" :key="view.id">
      <!-- Frame -->
      <div
        class="absolute border-gray-300 bg-gray-100"
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
          v-if="getViewComponent(view.type) != null"
          :is="getViewComponent(view.type)"
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
          draggingIdx == viewIdx - 1 ? 'bg-primary-400' : 'bg-transparent hover:bg-primary-300',
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
        @mousedown="draggingIdx = viewIdx - 1"
        data-outside-view="true"
      />
    </template>
    <!-- No frames -->
    <Empty
      v-if="sizedViews.length === 0"
      :type="type"
      class="flex h-full w-full flex-col items-center justify-center"
    />
  </div>
</template>

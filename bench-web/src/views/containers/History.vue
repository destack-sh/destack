<script lang="ts" setup>
import { supergraph } from "@/globals";
import { LOADED_PACKAGE_NODE_TYPES } from "@/language/core/const";
import { NodeType, RectangleData, ViewData } from "@/proto/wire";
import { toNodeRef, TypedNodeReferenceData } from "@/proto/wiring";
import { canvas, spaceGraph } from "@/system/space";
import { CommandMapKit } from "@/ui/command";
import { HISTORY_STATE_KEY, HistoryState } from "@/ui/view";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import { getViewBinding, getViewComponent } from "@/views/registry";
import { computed, provide, Ref, toRef } from "vue";

const props = defineProps<
  {
    self: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    size: Required<Pick<RectangleData, "width" | "height">>;
    isRoot?: boolean;
  } & Pick<ViewData, "icon" | "focusPtr" >
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
canvas.registerView(self, id);

const views = spaceGraph.getChildrenRef(self, NodeType.VIEW, { ignoreAncestors: true });
const focusedViewIdx: Ref<number | null> = computed(() => {
  if (views.value.length == 0) {
    return null;
  } else if (props.focusPtr != null) {
    const focusedId = props.focusPtr.id;
    const focusedTabIdx = views.value.findIndex((tab) => tab.id == focusedId);
    return focusedTabIdx >= 0 ? focusedTabIdx : views.value.length - 1;
  } else {
    return 0;
  }
});
const focusedView = computed(() => views.value[focusedViewIdx.value ?? 0]);

const history: HistoryState = {
  history: views,
  focusedViewIdx,
  focusedView,
  canGoBackward: computed(() => (focusedViewIdx.value ?? 0) > 0),
  canGoForward: computed(() => (focusedViewIdx.value ?? 0) < views.value.length - 1),
  go: (delta: number) => {
    // try to go up to delta times (skipping views where we don't have the node anymore)
    if (delta == 0 || focusedViewIdx.value == null) return;
    let viewIdx = focusedViewIdx.value;
    while (delta != 0) {
      viewIdx += Math.sign(delta);
      const view = views.value[viewIdx];
      if (view == null) {
        return; // view doesn't exist anymore
      }
      const viewNodePtr = view.nodePtr;
      if (
        viewNodePtr != null &&
        supergraph.get(viewNodePtr) == null &&
        LOADED_PACKAGE_NODE_TYPES.includes(viewNodePtr.nodeType)
      ) {
        continue; // node does not exist anymore (probably?)
      }
      delta -= Math.sign(delta);
    }
    const view = views.value[viewIdx];
    if (view == null) return;
    canvas.focus({ node: view });
  },
};
provide(HISTORY_STATE_KEY, history);

// actions
const commands: Partial<CommandMapKit<"view">> = {
  "view.history.goBackward": () => history.go(-1),
  "view.history.goForward": () => history.go(1),
};

defineExpose<ViewExpose>({ self, commands });
</script>
<template>
  <div class="h-full w-full">
    <component
      :is="getViewComponent(focusedView.type)"
      v-if="focusedView != null && getViewComponent(focusedView.type) != null"
      :key="focusedView.id"
      :self="toNodeRef(focusedView)"
      :is-root="isRoot"
      v-bind="getViewBinding(focusedView, size)"
    />
  </div>
</template>

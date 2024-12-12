<script lang="ts" setup>
import { NodeType, RectangleData, ViewData } from "@/proto/wire";
import { toNodeRef, TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection } from "@/system/connection";
import { canvas, pkgGraph } from "@/system/space";
import { ActionMapImplementation } from "@/ui/action";
import { HISTORY_STATE_KEY, HistoryState } from "@/ui/view";
import { viewEmits, type ViewExposed } from "@/views/common";
import { getViewBinding, getViewComponent } from "@/views/registry";
import { useEventListener } from "@vueuse/core";
import { computed, provide, Ref, toRef } from "vue";

const props = defineProps<
  {
    self: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    size: Required<Pick<RectangleData, "width" | "height">>;
  } & Pick<ViewData, "icon" | "nodePtr" | "focus" | "subnodePacked">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const views = spaceGraph.getChildrenRef(self, NodeType.VIEW, { ignoreAncestors: true });
const focusedViewIdx: Ref<number | null> = computed(() => {
  if (views.value.length == 0) {
    return null;
  } else if ((props.focus?.nodesPtr.length ?? 0) > 0) {
    const focusedId = props.focus!.nodesPtr[0].id;
    const focusedTabIdx = views.value.findIndex((tab) => tab.id == focusedId);
    return focusedTabIdx >= 0 ? focusedTabIdx : 0;
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
      if (view == null) return;
      const viewNodePtr = view.nodePtr;
      if (viewNodePtr != null && !pkgGraph.has(viewNodePtr)) continue;
      delta -= Math.sign(delta);
    }
    const view = views.value[viewIdx];
    if (view == null) return;
    canvas.focus({ node: view });
  },
};
provide(HISTORY_STATE_KEY, history);

// actions
const actions: Partial<ActionMapImplementation<"view">> = {
  "view.history.goBackward": () => history.go(-1),
  "view.history.goForward": () => history.go(1),
};

defineExpose<ViewExposed>({ self, actions });
</script>
<template>
  <div class="h-full w-full">
    <component
      :is="getViewComponent(focusedView.type)"
      v-if="focusedView != null && getViewComponent(focusedView.type) != null"
      :self="toNodeRef(focusedView)"
      v-bind="getViewBinding(focusedView, size)"
    />
  </div>
</template>

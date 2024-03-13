<script lang="tsx" setup>
import { BenchType, BoxData, NodeReferenceData, NodeType, Orientation, SelectionKind, ViewData } from "@/proto/wire";
import { toNodeReference } from "@/proto/wiring";
import { useLoadedGraph } from "@/system/connection";
import { IconInline } from "@/system/icon";
import { updateOrder } from "@/system/lang";
import { setDragData, useMultiDropZone, useSingleDropZone } from "@/utils/drag";
import { log } from "@/utils/log";
import { getViewBinding, getViewComponent } from "@/views";
import { viewEmits } from "@/views/common";
import { computed, toRef, type Ref, ref } from "vue";

const props = defineProps<
  { self: NodeReferenceData; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "name" | "title" | "text" | "icon" | "selection"
  >
>();
const emit = defineEmits(viewEmits());

// selection
const { graph: spaceGraph, connection: spaceConnection } = useLoadedGraph(toRef(props, "self"));
const tabs = spaceGraph.getChildrenRef(toRef(props, "self"), NodeType.VIEW);

const selectedTabIdx: Ref<number | null> = computed(() => {
  if (tabs.value.length == 0) return null;
  if ((props.selection?.nodesPtr.length ?? 0) > 0) {
    const selectedId = props.selection!.nodesPtr[0].id;
    const selectedTabIdx = tabs.value.findIndex((tab) => tab.id == selectedId);
    return selectedTabIdx >= 0 ? selectedTabIdx : 0;
  } else {
    return 0;
  }
});
const innerSize = computed(() => ({
  width: props.size.width,
  height: props.size.height - 30,
}));

function select(tab: ViewData) {
  log.debug("tabbed.select", tab);
  spaceConnection.sideTx.update({
    metatype: NodeType.VIEW,
    id: props.self.id,
    selection: {
      metatype: BenchType.SELECTION,
      kind: SelectionKind.LIST,
      nodesPtr: [toNodeReference(tab)],
    },
  });
}

function remove(tab: ViewData) {
  log.debug("tabbed.remove", tab);
  spaceConnection.sideTx.delete(tab); // should soft delete?
}

// dragging
const tabHeaderRef: Ref<HTMLElement | null> = ref(null);
const tabsRef: Ref<Record<string, HTMLElement>> = ref({});
const { activeDropZone } = useMultiDropZone({
  container: tabHeaderRef,
  targets: tabsRef,
  kinds: ["node"],
  metatypes: [NodeType.VIEW],
  orientation: Orientation.HORIZONTAL,
  onDrop: (dragged, anchor, targetId) => {
    if (dragged.kind == "node") {
      log.debug("tabbed.drop", props.self, dragged.node, anchor, targetId);
      const draggedNode = spaceGraph.get(dragged.node) as ViewData;
      if (draggedNode != null) {
        // move & update order
        if (draggedNode.id != targetId) {
          const targetNode = tabs.value.find((tab) => tab.id == targetId)!;
          updateOrder({
            tx: spaceConnection.sideTx,
            target: draggedNode,
            position: anchor == "start" ? "before" : "after",
            reference: targetNode,
            nodes: tabs.value,
          });
        }
        if (draggedNode.parentPtr?.id != props.self.id) {
          spaceConnection.sideTx.move({ ...draggedNode, parentPtr: props.self });
        }
        select(draggedNode);
      }
    }
  },
});

defineExpose({ self: toRef(props, "self"), select, remove });
</script>
<template>
  <div class="relative" :style="{ width: size.width + 'px', height: size.height + 'px' }">
    <!-- Tab header -->
    <div
      ref="tabHeaderRef"
      class="flex h-[30px] w-full flex-row overflow-x-scroll border-b-2 border-gray-300"
      :class="[activeDropZone != null ? 'bg-gray-100' : 'bg-gray-200']"
    >
      <!-- Tab button -->
      <button
        :ref="(ref) => (ref != null ? (tabsRef[tab.id] = ref as HTMLElement) : delete tabsRef[tab.id])"
        v-for="(tab, i) in tabs"
        :key="tab.id"
        class="group relative flex h-full max-w-52 flex-row items-center justify-center whitespace-nowrap border-r-2 border-gray-300 bg-gray-100 px-2.5 hover:cursor-pointer"
        :class="[
          i == selectedTabIdx
            ? 'text-primary-900 shadow-inset-md shadow-primary-900'
            : 'text-gray-700 hover:text-primary-900',
        ]"
        @click="select(tab)"
        :draggable="true"
        @dragstart="
          (e: DragEvent) => {
            setDragData(e, { kind: 'node', node: toNodeReference(tab) });
            e.dataTransfer?.setDragImage(tabsRef[tab.id]!, 0, 0)
          }
        "
      >
        <IconInline
          v-if="tab.icon"
          v-bind="tab.icon"
          class="mr-1.5"
          :class="i == selectedTabIdx ? '' : 'text-gray-600 group-hover:text-primary-900'"
        />
        <span class="truncate" :class="[tab.title ? '' : 'italic', i == selectedTabIdx ? '' : '']">
          {{ tab.title ?? `Tab ${i + 1}` }}
        </span>
        <!-- Close tab button -->
        <button
          class="ml-1.5 group-hover:text-gray-400"
          :class="[i == selectedTabIdx ? 'text-gray-400' : 'text-transparent']"
          @click.stop="remove(tab)"
        >
          <i class="fas fa-xmark hover:text-primary-900" />
        </button>
        <!-- Drop indicator -->
        <div
          v-if="activeDropZone?.targetId == tab.id"
          class="absolute z-10 h-full w-1 bg-primary-400"
          :class="[activeDropZone.anchor == 'start' ? '-left-[3px]' : '-right-[3px]']"
        />
      </button>
    </div>
    <!-- Tab body -->
    <div class="absolute" :style="{ width: innerSize.width + 'px', height: innerSize.height + 'px' }">
      <!-- Content -->
      <component
        v-if="selectedTabIdx != null && getViewComponent(tabs[selectedTabIdx].type) != null"
        :is="getViewComponent(tabs[selectedTabIdx].type)"
        :self="toNodeReference(tabs[selectedTabIdx])"
        v-bind="getViewBinding(tabs[selectedTabIdx], innerSize)"
      />
      <div v-else-if="selectedTabIdx != null" class="h-full w-full">
        <!-- missing view -->
      </div>
      <div v-else class="h-full w-full">
        <!-- empty state -->
      </div>
    </div>
  </div>
</template>

<script lang="tsx" setup>
import { BenchType, BoxData, NodeReferenceData, NodeType, SelectionKind, ViewData } from "@/proto/wire";
import { toNodeReference } from "@/proto/wiring";
import { useLoadedGraph } from "@/system/connection";
import { IconInline } from "@/system/icon";
import { log } from "@/utils/log";
import { getViewBinding, getViewComponent } from "@/views";
import { viewEmits } from "@/views/common";
import { computed, toRef, type Ref } from "vue";

const props = defineProps<
  { self: NodeReferenceData; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "name" | "title" | "text" | "icon" | "selection"
  >
>();
const emit = defineEmits(viewEmits());

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
  spaceConnection.sideTx.delete(tab);
}

defineExpose({ self: toRef(props, "self") });
</script>
<template>
  <div class="relative" :style="{ width: size.width + 'px', height: size.height + 'px' }">
    <!-- Tab header -->
    <div class="flex h-[30px] w-full flex-row overflow-x-scroll border-b-2 border-gray-300 bg-gray-200">
      <button
        v-for="(tab, i) in tabs"
        :key="tab.id"
        class="group flex max-w-52 h-full flex-row items-center justify-center whitespace-nowrap bg-gray-100 px-2.5"
        :class="[
          i == selectedTabIdx ? 'shadow-inset-sm text-primary-900 shadow-primary-900' : 'hover:text-primary-900',
          'border-r-2 border-gray-300',
        ]"
        @click="select(tab)"
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
        <button
          class="ml-1.5 group-hover:text-gray-300"
          :class="[i == selectedTabIdx ? 'text-gray-300' : 'text-transparent']"
          @click.stop="remove(tab)"
        >
          <i class="fas fa-xmark hover:text-primary-900" />
        </button>
      </button>
    </div>
    <!-- Tab content -->
    <div class="absolute" :style="innerSize">
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

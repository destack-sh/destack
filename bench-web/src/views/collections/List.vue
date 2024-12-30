<script lang="ts" setup>
import { isBenchNodeType } from "@/language/const";
import { makeExpression } from "@/language/expression";
import { useSubnodeProperty } from "@/language/node";
import { AnyNodeData, ExpressionType, NodeType, RecordProperty, ViewData, ViewType } from "@/proto/wire";
import { EMPTY_SCOPE, propertyReference, TypedNodeReferenceData } from "@/proto/wiring";
import { BENCH_SCOPE } from "@/system/client";
import { useSearchConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { startSelectingIfAllowed, useSelectionZone } from "@/ui/drag";
import { getNodeIcon, IconInline } from "@/ui/icon";
import NodeMetadata from "@/views/builtins/NodeMetadata.vue";
import SelectionOverlay from "@/views/builtins/SelectionOverlay.vue";
import { viewEmits, type ViewExposed } from "@/views/common";
import { computed, Ref, ref, toRef } from "vue";

const ITEM_HEIGHT = 28;

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Partial<Pick<ViewData, "subnodePacked">>
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

// filter
const nodeType = useSubnodeProperty(NodeType.VIEW, ViewType.LIST, toRef(props, "subnodePacked"), "queryNodeType");
const filter = useSubnodeProperty(NodeType.VIEW, ViewType.LIST, toRef(props, "subnodePacked"), "filter");

// search
const DEFAULT_SORT = makeExpression({
  type: ExpressionType.ASCENDING,
  propertyPtr: propertyReference(NodeType.RECORD, RecordProperty.createdAt),
});
const { connection, graph, page, roots, isConnecting, isStale } = useSearchConnection(
  { name: "list", live: true },
  computed(() => ({
    nodeType: nodeType.value!,
    scope: nodeType.value != null && isBenchNodeType(nodeType.value) ? BENCH_SCOPE.value : EMPTY_SCOPE,
    filter: filter.value,
    isEnabled: nodeType.value != null,
  })),
);
const total = computed(() => page.value?.total);

// interaction
const containerRef = ref<HTMLElement | null>(null);
const itemRefs = ref<Record<string, HTMLElement>>({});
const selectionOverlayRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionZone = useSelectionZone({ containerEl: containerRef, overlayEl: selectionOverlayRef });

defineExpose<ViewExposed & { total: Ref<number | undefined>; roots: Ref<AnyNodeData[]> }>({
  self,
  id,
  total,
  roots,
});
</script>
<template>
  <div ref="containerRef" @mousedown="(e) => startSelectingIfAllowed(selectionZone, e)">
    <!-- List -->
    <ul class="relative flex flex-col">
      <li
        v-for="node in roots"
        :ref="(ref?: any) => (ref != null ? (itemRefs[node.id] = ref) : delete itemRefs[node.id])"
        :key="node.id"
        class="group/node relative mx-1.5 flex flex-row items-center px-2.5 transition-colors duration-150 hover:cursor-pointer"
        :class="[
          canvas.isSelected(node)
            ? 'bg-orange-400/20'
            : canvas.isHighlighted(node)
              ? 'bg-gray-100'
              : 'hover:bg-gray-100',
        ]"
        :data-node-id="node.id"
        :data-node-ck="(node as any).ck"
        :data-node-type="node.metatype"
        :style="{
          height: ITEM_HEIGHT + 'px',
        }"
        data-suppress-drag="select"
        @click="canvas.goToNode(node)"
      >
        <!-- Icon -->
        <IconInline
          v-bind="getNodeIcon(node)"
          class="mr-1.5 w-5 text-center text-gray-700 transition-colors duration-75"
        />
        <span class="max-w-full select-none truncate">
          {{ (node as any).slug ?? (node as any).title ?? (node as any).name ?? "???" }}
        </span>
        <!-- Metadata -->
        <NodeMetadata class="ml-1.5" size="regular" :node="node" />
      </li>
      <!-- Empty -->
      <div
        v-if="total == 0"
        class="mx-1.5 flex flex-row items-center px-2.5"
        :style="{
          height: ITEM_HEIGHT + 'px',
        }"
      >
        <!-- Loading -->
        <span v-if="isConnecting" class="">
          <i class="fas fa-spinner-third animate-spin text-gray-400" />
        </span>
        <!-- Empty -->
        <span v-else class="text-gray-400">No results</span>
      </div>

      <!-- Selection overlay -->
      <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
    </ul>
  </div>
</template>

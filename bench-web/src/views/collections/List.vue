<script lang="ts" setup>
import { isBenchNodeType, toCamelName } from "@/language/core/const";
import { makeExpression } from "@/language/core/expression";
import { useSubnodeProperty } from "@/language/core/node";
import { AnyNodeData, ExpressionType, NodeType, RecordProperty, ViewData, ViewType } from "@/proto/wire";
import { EMPTY_SCOPE, propertyReference, TypedNodeReferenceData } from "@/proto/wiring";
import { BENCH_SCOPE } from "@/system/client";
import { useSearchConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { CONTEXT_ACTIONS_BY_TYPE, getAction, isActionEnabled } from "@/ui/action";
import { startSelectingIfAllowed, useSelectionZone } from "@/ui/drag";
import { getNodeIcon, getNodeTitle, IconInline } from "@/ui/icon";
import NodeMetadata from "@/views/builtins/NodeMetadata.vue";
import SelectionOverlay from "@/views/overlays/SelectionOverlay.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import { computed, Ref, ref, toRef } from "vue";

const ITEM_HEIGHT = 28;

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Partial<Pick<ViewData, "subnodePacked">>
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

// filter
const nodeType = useSubnodeProperty(NodeType.VIEW, ViewType.LIST, toRef(props, "subnodePacked"), "queryNodeType");
const filter = useSubnodeProperty(NodeType.VIEW, ViewType.LIST, toRef(props, "subnodePacked"), "filter");
const nodeActions = computed(() =>
  nodeType.value != null ? CONTEXT_ACTIONS_BY_TYPE[nodeType.value]?.map(getAction) : [],
);

// search
const DEFAULT_SORT = makeExpression({
  type: ExpressionType.DESCENDING,
  propertyPtr: propertyReference(NodeType.RECORD, RecordProperty.createdAt),
});
const { connection, graph, page, roots, isConnecting, isStale } = useSearchConnection(
  { name: "list", live: true },
  computed(() => ({
    nodeType: nodeType.value!,
    scope: nodeType.value != null && isBenchNodeType(nodeType.value) ? BENCH_SCOPE.value : EMPTY_SCOPE,
    sort: [DEFAULT_SORT],
    filter: filter.value,
    isEnabled: nodeType.value != null,
    first: 20,
  })),
);
const total = computed(() => page.value?.total);

// interaction
const containerRef = ref<HTMLElement | null>(null);
const itemRefs = ref<Record<string, HTMLElement>>({});
const selectionOverlayRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionZone = useSelectionZone({ containerEl: containerRef, overlayEl: selectionOverlayRef });

defineExpose<ViewExpose & { total: Ref<number | undefined>; roots: Ref<AnyNodeData[]> }>({
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
        role="button"
        @click.stop="canvas.goToNode(node)"
      >
        <!-- Icon -->
        <IconInline
          v-bind="getNodeIcon(node)"
          class="mr-1 w-5 text-center text-gray-700 transition-colors duration-75"
        />
        <span class="max-w-full select-none truncate">{{ getNodeTitle(node) ?? "???" }}</span>
        <!-- Metadata -->
        <NodeMetadata class="ml-1.5" size="regular" :node="node" />
        <!-- Actions -->
        <div class="ml-auto flex flex-row gap-x-1">
          <button
            v-for="action of nodeActions?.filter((a) => isActionEnabled(a, { nodes: [node] }))"
            :key="action.id"
            v-tooltip="{ small: true, text: action.title, group: 'list.item' }"
            aria-hidden
            class="rounded-sm px-1 py-0.5 text-gray-400 opacity-0 transition-colors duration-150 hover:bg-gray-100 hover:text-gray-700 group-hover/node:opacity-100"
            @click.stop="(e) => action.action?.(action, { event: e, nodes: [node] })"
          >
            <IconInline v-bind="action.icon" />
          </button>
        </div>
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
        <span v-else class="text-gray-400">No {{ toCamelName(NodeType, nodeType) }}s</span>
      </div>

      <!-- Selection overlay -->
      <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
    </ul>
  </div>
</template>

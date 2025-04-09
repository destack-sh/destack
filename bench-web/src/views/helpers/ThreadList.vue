<script lang="ts" setup>
import { toCamelName } from "@/language/core/const";
import { makeExpression } from "@/language/core/expression";
import { AnyNodeData, ExpressionType, NodeType, TextLineType, ThreadProperty, ViewData } from "@/proto/wire";
import { propertyReference, TypedNodeReferenceData } from "@/proto/wiring";
import { CURRENT_BENCH_SCOPE } from "@/system/client";
import { useSearchConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { CONTEXT_COMMANDS_BY_TYPE, getCommand, isCommandEnabled } from "@/ui/command";
import { startSelectingIfAllowed, useSelectionZone } from "@/ui/drag";
import { getNodeIcon, IconInline } from "@/ui/icon";
import NodeMetadata from "@/views/builtin/NodeMetadata.vue";
import Title from "@/views/builtin/Title.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import SelectionOverlay from "@/views/overlays/SelectionOverlay.vue";
import { computed, Ref, ref, toRef } from "vue";

const ITEM_HEIGHT = 30;
const DEFAULT_SORT = [
  makeExpression({
    type: ExpressionType.DESCENDING,
    propertyPtr: propertyReference(NodeType.THREAD, ThreadProperty.activeAt),
  }),
  makeExpression({
    type: ExpressionType.DESCENDING,
    propertyPtr: propertyReference(NodeType.THREAD, ThreadProperty.createdAt),
  }),
];
const NODE_COMMANDS = CONTEXT_COMMANDS_BY_TYPE[NodeType.THREAD]?.map(getCommand);

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
  } & Partial<Pick<ViewData, "subnodePacked">>
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");

// search
const {
  connection,
  graph,
  page,
  roots: threads,
  isConnecting,
  isStale,
} = useSearchConnection(
  { name: "list", live: true },
  computed(() => ({
    nodeType: NodeType.THREAD,
    scope: CURRENT_BENCH_SCOPE.value,
    sort: DEFAULT_SORT,
    isEnabled: true,
    first: 20,
  })),
);
const total = computed(() => page.value?.total);

// interaction
const containerRef = ref<HTMLElement | null>(null);
const itemRefs = ref<Record<string, HTMLElement>>({});
const selectionOverlayRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionZone = useSelectionZone({ containerEl: containerRef, overlayEl: selectionOverlayRef });

defineExpose<Omit<ViewExpose, "id"> & { total: Ref<number | undefined>; roots: Ref<AnyNodeData[]> }>({
  self,
  total,
  roots: threads,
});
</script>
<template>
  <div ref="containerRef" @mousedown="(e) => startSelectingIfAllowed(selectionZone, e)">
    <!-- List -->
    <ul class="relative flex flex-col">
      <li
        v-for="thread in threads"
        :ref="(ref?: any) => (ref != null ? (itemRefs[thread.id] = ref) : delete itemRefs[thread.id])"
        :key="thread.id"
        class="group/node relative mx-1.5 flex max-w-full flex-row items-center rounded px-2.5 transition-colors duration-150 hover:cursor-pointer"
        :class="[
          canvas.isSelected(thread)
            ? 'bg-orange-400/20'
            : canvas.isHighlighted(thread)
              ? 'bg-gray-100'
              : 'hover:bg-gray-100',
        ]"
        :data-node-type="thread.metatype"
        :data-node-id="thread.id"
        :data-node-ck="(thread as any).ck"
        :data-node-bench-id="(thread as any).benchPtr?.id"
        :style="{
          height: ITEM_HEIGHT + 'px',
        }"
        data-suppress-drag="select"
        role="button"
        @click.stop="canvas.goToNode(thread)"
      >
        <!-- Icon -->
        <IconInline
          v-bind="getNodeIcon(thread)"
          class="mr-1 w-5 text-center text-gray-700 transition-colors duration-75"
        />
        <!-- Name -->
        <Title
          :model-value="thread.title"
          :force-line-type="TextLineType.PARAGRAPH"
          class="max-w-full select-none truncate"
          truncate
          is-small
          :placeholder="toCamelName(NodeType, thread.metatype)"
        />
        <!-- Meta -->
        <div class="ml-auto flex flex-row gap-x-1">
          <!-- Metadata -->
          <NodeMetadata class="ml-1.5" size="sm" :node="thread" />
          <button
            v-for="command of NODE_COMMANDS?.filter((c) => isCommandEnabled(c, { nodes: [thread] }))"
            :key="command.id"
            v-tooltip="{ small: true, text: command.title, group: 'list.item' }"
            aria-hidden
            class="rounded-sm px-1 py-0.5 text-gray-400 opacity-0 transition-colors duration-150 hover:bg-gray-100 hover:text-gray-700 group-hover/node:opacity-100"
            @click.stop="(e) => command.command?.(command, { event: e, nodes: [thread] })"
          >
            <IconInline v-bind="command.icon" />
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
        <!-- Empty -->
        <span v-if="!isConnecting" class="text-gray-400">No Threads</span>
      </div>

      <!-- Selection overlay -->
      <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
    </ul>
  </div>
</template>

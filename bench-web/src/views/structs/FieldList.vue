<script lang="ts" setup>
import { blockToType } from "@/language/block";
import { toCamelName, TYPE_BLOCK_TYPES } from "@/language/const";
import { createField } from "@/language/field";
import { useNodeListActions } from "@/ui/list";
import { moveNode, onNodeMorphed } from "@/language/node";
import { newChangeId } from "@/language/transaction";
import { BlockType, FieldType, NodeType, Orientation, ViewData, type FieldData } from "@/proto/wire";
import { describeNode, isNode, toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { FIELD_CONTEXT_ACTIONS, type ActionMapImplementation } from "@/ui/action";
import { onAddFieldAction } from "@/ui/detail";
import {
  isDragging,
  startDraggingIfAllowed,
  startSelectingIfAllowed,
  useMultiDropZone,
  useSelectionZone,
  type DragContent,
  type MultiAnchor,
} from "@/ui/drag";
import SelectionOverlay from "@/views/builtins/SelectionOverlay.vue";
import { viewEmits, type ViewExposed } from "@/views/common";
import Field from "@/views/nodes/Field.vue";
import { computed, ref, toRef, type Ref } from "vue";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    preparedConnection?: PreparedGetConnection;
    fieldType: FieldType;
  } & Partial<Pick<ViewData, "isMinimal" | "orientation" | "nodePtr">>
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const orientation = computed(() => props.orientation ?? Orientation.HORIZONTAL);
const isHorizontal = computed(
  () => orientation.value == Orientation.HORIZONTAL || orientation.value == Orientation.HORIZONTAL_REVERSED,
);
const state = canvas.registerView(self, id);

const containerRef = ref<HTMLElement | null>(null);
const fieldRefs: Ref<Record<string, InstanceType<typeof Field> | null>> = ref({});

const nodePtr = computed(() => props.nodePtr as TypedNodeReferenceData<NodeType.BLOCK>);
const { graph, connection } = props.preparedConnection ?? useExistingConnection(nodePtr);
const block = graph.getRef(nodePtr, { ignoreAncestors: props.self == null });
const allFields = graph.getChildrenRef(block, NodeType.FIELD);
const fields = computed(() => allFields.value.filter((f) => f.type == props.fieldType));

// dragging :TypeDragAndDrop
// NOTE: we have separate drop types for left/right (for function types)
function allowDrop(dragged: DragContent, anchor: MultiAnchor, targetId: string | null, event?: DragEvent): boolean {
  if (dragged.kind != "node" && dragged.kind != "selection") return false;
  return dragged.nodes.every((node) => {
    node = graph.getOrError(node);
    if (isNode(node, NodeType.FIELD) && (node.type == FieldType.OPTION) == (block.value?.type == BlockType.CHOICE)) {
      return true;
    } else if (
      isNode(node, NodeType.BLOCK) &&
      block.value?.type != BlockType.CHOICE &&
      TYPE_BLOCK_TYPES.includes(node.type)
    ) {
      return true;
    } else {
      return false;
    }
  });
}
function onDrop(dragged: DragContent, anchor: MultiAnchor, targetId: string | null, event: DragEvent) {
  if (dragged.kind != "node" && dragged.kind != "selection") return;
  const tx = connection.tx.with({ change: { key: newChangeId(), title: "Move" } });
  const target = targetId != null ? graph.get({ id: targetId }) : null;
  for (let i = 0; i < dragged.nodes.length; i++) {
    const node = graph.getOrError(dragged.nodes[i]);
    if (isNode(node, NodeType.FIELD)) {
      // move field
      if (target != null) {
        if (!isNode(target, NodeType.FIELD)) throw new Error(`unexpected target node: ${describeNode(target)}`);
        moveNode(tx, graph, node, {
          anchor: i == 0 ? anchor : "after",
          target: i == 0 ? target : graph.getOrError(dragged.nodes[i - 1]),
        });
      } else {
        moveNode(tx, graph, node, { anchor: "center", target: block.value! });
      }
      if (node.type != props.fieldType) {
        tx.update(node, { type: props.fieldType ?? undefined }, { debounce: "tick" });
        onNodeMorphed(tx, graph, node);
      }
    } else if (isNode(node, NodeType.BLOCK)) {
      // add field with block type
      const type = blockToType(node);
      const fieldIn = { ...type, type: props.fieldType! };
      if (target != null) {
        if (!isNode(target, NodeType.FIELD)) throw new Error(`unexpected target node: ${describeNode(target)}`);
        createField(tx, graph, {
          field: fieldIn,
          anchor: i == 0 ? anchor : "after",
          target: i == 0 ? target : (graph.getOrError(dragged.nodes[i - 1]) as FieldData),
        });
      } else {
        createField(tx, graph, { field: fieldIn, anchor: "inside", target: block.value! });
      }
    }
  }
}
const { activeDropZone } = useMultiDropZone({
  name: "type",
  container: containerRef,
  targets: fieldRefs,
  orientation: orientation.value,
  kinds: ["node", "selection"],
  metatypes: [NodeType.BLOCK, NodeType.FIELD],
  fallbackToClosest: true,
  allowDrop,
  onDrop,
});

// selecting
const selectionOverlayRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionZone = useSelectionZone({ containerEl: containerRef, overlayEl: selectionOverlayRef });

// actions
// NOTE :Incomplete: Type.actions (move, navigate, ...)
const actions: Partial<ActionMapImplementation<"list">> = {
  // list
  ...useNodeListActions({
    nodeType: NodeType.FIELD,
    self: state.baseViewRef,
    graph,
    list: fields,
    txFactory: () => connection.tx,
    create: (anchor, node) =>
      createField(connection.tx, graph, {
        anchor: node != null ? anchor : "inside",
        target: node ?? block.value!,
        field: { type: props.fieldType },
      }),
  }),
};

defineExpose<ViewExposed>({ self, id, actions });
</script>
<template>
  <div
    ref="containerRef"
    class="relative"
    :class="[activeDropZone != null ? 'outline outline-2 outline-gray-400' : '']"
  >
    <!-- NOTE :UX: field type drop outline should be dotted if dragged is not a field
        (since it's not a move, but a sort of 'copy', and that's how we telegraph it elsewhere) -->
    <!-- Drop indicator -->
    <div v-if="activeDropZone" class="absolute right-1 top-1 text-gray-400">
      {{ toCamelName(FieldType, props.fieldType) }}
    </div>
    <ul
      class="flex gap-x-2 gap-y-1 rounded"
      :class="[isHorizontal ? 'flex-row' : 'flex-col', isMinimal ? 'px-0.5 py-0.5' : '']"
      @mousedown="(e) => startSelectingIfAllowed(selectionZone, e)"
    >
      <!-- Field wrapper -->
      <li
        v-for="field in fields"
        :key="field.id"
        class="relative w-fit"
        :class="!isHorizontal ? 'w-full' : 'max-w-[200px]'"
      >
        <!-- Drop indicator -->
        <div
          v-if="activeDropZone?.targetId == field.id"
          class="absolute z-10 rounded bg-gray-400"
          :class="[
            isHorizontal ? 'h-full w-1' : 'h-1 w-full',
            activeDropZone?.anchor == 'start'
              ? isHorizontal
                ? '-left-[6px]'
                : '-top-[3px]'
              : isHorizontal
                ? '-right-[6px]'
                : '-bottom-[3px]',
          ]"
        />
        <!-- Field -->
        <Field
          :id="field.id"
          :ref="(ref: any) => (ref != null ? (fieldRefs[field.id] = ref) : delete fieldRefs[field.id])"
          class="cursor-pointer truncate transition-colors duration-150"
          :class="[
            orientation == Orientation.VERTICAL ? 'w-full' : 'max-w-[200px]',
            isDragging(field) ? 'opacity-50' : '',
          ]"
          :data-contextmenu-items="FIELD_CONTEXT_ACTIONS.join(',')"
          :prepared-connection="preparedConnection"
          :node-ptr="toNodeRef(field)"
          :is-minimal="isMinimal ?? false"
          :draggable="true"
          @dragstart.stop="(e: DragEvent) => startDraggingIfAllowed(e, field)"
        />
      </li>
      <!-- Add button -->
      <button
        v-if="!isMinimal || fields.length == 0"
        class="h-[28px] rounded px-1 text-left text-gray-400 transition-colors duration-150 hover:bg-gray-100 hover:text-gray-700"
        :class="isHorizontal ? '' : 'mx-1.5'"
        @click="(e) => onAddFieldAction(e, fieldType, block!, graph, () => connection.tx)"
      >
        <i class="fas fa-plus mr-1.5" />
        <span> {{ toCamelName(FieldType, props.fieldType) }} </span>
      </button>
    </ul>
    <!-- Selection overlay -->
    <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
  </div>
</template>

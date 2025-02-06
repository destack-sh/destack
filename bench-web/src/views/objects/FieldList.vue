<script lang="ts" setup>
import { toCamelName } from "@/language/core/const";
import { createField, useFieldList } from "@/language/source/field";
import { FieldType, NodeType, Orientation, ViewData } from "@/proto/wire";
import { toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { FIELD_CONTEXT_ACTIONS, type ActionMapImplementation } from "@/ui/action";
import {
  isDragging,
  startDraggingIfAllowed,
  startSelectingIfAllowed,
  useMultiDropZone,
  useSelectionZone
} from "@/ui/drag";
import { useNodeListActions } from "@/ui/list";
import { onAddFieldAction } from "@/ui/object";
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
const { allowDrop, onDrop } = useFieldList({
  graph,
  txFactory: () => connection.tx,
  fieldType: toRef(props, "fieldType"),
  base: block,
});
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
            isMinimal ? 'px-1 py-[3px]' : '',
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

<script lang="ts" setup>
import { blockToType } from "@/language/block";
import { toCamelName, TYPE_BLOCK_TYPES } from "@/language/const";
import { createField, FIELD_CONTEXT_ACTIONS, makeTypeInfo, TypeIdentity } from "@/language/field";
import { cloneNode, moveNode, onNodeMorphed } from "@/language/node";
import {
  BenchType,
  BlockType,
  FieldType,
  NodeType,
  Orientation,
  ViewData,
  ViewType,
  type FieldData,
} from "@/proto/wire";
import { describeNode, isNode, toNodeRefOneOf, unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import type { ActionContext, ActionMapImplementation } from "@/ui/action";
import { startDraggingIfAllowed, useMultiDropZone, type DraggedContent, type MultiAnchor } from "@/ui/drag";
import { menuActionsLike, pushPopover, type PopoverContext, type PopoverInfo } from "@/ui/popover";
import { viewEmits, type ViewExposed } from "@/views/common";
import Field from "@/views/system/Field.vue";
import { computed, ref, toRef, type Ref } from "vue";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    preparedConnection?: PreparedGetConnection;
    fieldType: FieldType;
  } & Partial<Pick<ViewData, "name" | "variant" | "orientation" | "nodePtr">>
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const orientation = computed(() => props.orientation ?? Orientation.HORIZONTAL);

const containerRef = ref<HTMLElement | null>(null);
const fieldRefs: Ref<Record<string, InstanceType<typeof Field> | null>> = ref({});

const nodePtr = computed(() => unwrapProtoOneOf(props.nodePtr) as TypedNodeReferenceData<NodeType.BLOCK>);
const { graph: pkgGraph, connection: pkgConnection } = props.preparedConnection ?? useExistingConnection(nodePtr);
const block = pkgGraph.getRef(nodePtr, { ignoreAncestors: props.self == null });
const allFields = pkgGraph.getChildrenRef(block, NodeType.FIELD);
const fields = computed(() => allFields.value.filter((f) => f.type == props.fieldType));

// dragging :TypeDragAndDrop
// NOTE: we have separate drop types for left/right (for function types)
function allowDrop(dragged: DraggedContent, anchor: MultiAnchor, targetId: string | null, event?: DragEvent): boolean {
  if (dragged.kind != "node") return false;
  const node = pkgGraph.get(dragged.node);
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
}
function onDrop(dragged: DraggedContent, anchor: MultiAnchor, targetId: string | null, event: DragEvent) {
  if (dragged.kind != "node") return;
  const node = pkgGraph.getOrError(dragged.node);
  const target = targetId != null ? pkgGraph.get({ id: targetId }) : null;
  if (isNode(node, NodeType.FIELD)) {
    // move field
    if (target != null) {
      if (!isNode(target, NodeType.FIELD)) throw new Error(`unexpected target node: ${describeNode(target)}`);
      moveNode(pkgConnection.tx, pkgGraph, dragged.node, { anchor, target });
    } else {
      moveNode(pkgConnection.tx, pkgGraph, dragged.node, { anchor: "center", target: block.value! });
    }
    if (node.type != props.fieldType) {
      pkgConnection.tx.update(node, { type: props.fieldType ?? undefined }, { debounce: "tick" });
      onNodeMorphed(pkgConnection.tx, pkgGraph, node);
    }
  } else if (isNode(node, NodeType.BLOCK)) {
    // add field with block type
    const type = blockToType(node);
    const fieldIn = { ...type, type: props.fieldType! };
    if (target != null) {
      if (!isNode(target, NodeType.FIELD)) throw new Error(`unexpected target node: ${describeNode(target)}`);
      createField(pkgConnection.tx, pkgGraph, { field: fieldIn, anchor, target });
    } else {
      createField(pkgConnection.tx, pkgGraph, { field: fieldIn, anchor: "inside", target: block.value! });
    }
  }
}
const { activeDropZone } = useMultiDropZone({
  name: "type",
  container: containerRef,
  targets: fieldRefs,
  orientation: orientation.value,
  kinds: ["node"],
  metatypes: [NodeType.BLOCK, NodeType.FIELD],
  fallbackToClosest: true,
  allowDrop,
  onDrop,
});

// actions
const getFieldFromContext = (ctx: ActionContext | undefined): { field: FieldData | null } => {
  const field = fields.value.find((f) => f.id == ctx?.triggerNode?.id) ?? null;
  // NOTE :Incomplete: Type fallback to focused/inspection/...? like in other actions?
  return { field };
};
// NOTE :Incomplete: Type.actions (move, navigate, ...)
const actions: Partial<ActionMapImplementation<"common">> = {
  // common
  "common.create.above": (action, ctx) => {
    const { field } = getFieldFromContext(ctx);
    if (field == null) return false;
    createField(pkgConnection.tx, pkgGraph, { anchor: "before", target: field, field: { type: props.fieldType } });
  },
  "common.create.below": (action, ctx) => {
    const { field } = getFieldFromContext(ctx);
    if (field == null) return false;
    createField(pkgConnection.tx, pkgGraph, { anchor: "after", target: field, field: { type: props.fieldType } });
  },
  "common.edit.duplicate": (action, ctx) => {
    const { field } = getFieldFromContext(ctx);
    if (field == null) return false;
    const duplicate = cloneNode(pkgConnection.tx, pkgGraph, field, { includeChildren: true });
  },
  "common.edit.delete": (action, ctx) => {
    const { field } = getFieldFromContext(ctx);
    if (field == null) return false;
    pkgConnection.tx.delete(field);
  },
};

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, actions });
</script>
<template>
  <ul
    ref="containerRef"
    class="relative flex gap-x-2 gap-y-1 rounded"
    :class="[
      orientation == Orientation.HORIZONTAL ? 'flex-row' : 'flex-col',
      activeDropZone != null ? 'outline outline-2 outline-primary-700' : '',
    ]"
  >
    <!-- NOTE :UX: field type drop outline should be dotted if dragged is not a field
        (since it's not a move, but a sort of 'copy', and that's how we signal it elsewhere) -->
    <!-- Drop indicator -->
    <div v-if="activeDropZone" class="absolute right-1 top-1 text-primary-700">
      {{ toCamelName(FieldType, props.fieldType) }}
    </div>
    <!-- Field wrapper -->
    <li v-for="field in fields" :key="field.id" class="relative w-fit max-w-[200px]">
      <!-- Drop indicator -->
      <div
        v-if="activeDropZone?.targetId == field.id"
        class="absolute z-10 rounded bg-primary-700"
        :class="[
          orientation == Orientation.HORIZONTAL ? 'h-full w-1' : 'h-1 w-full',
          activeDropZone?.anchor == 'start'
            ? orientation == Orientation.HORIZONTAL
              ? '-left-[6px]'
              : '-top-[3px]'
            : orientation == Orientation.HORIZONTAL
              ? '-right-[6px]'
              : '-bottom-[3px]',
        ]"
      />
      <!-- Field -->
      <Field
        :id="field.id"
        :ref="(ref: any) => (ref != null ? (fieldRefs[field.id] = ref) : delete fieldRefs[field.id])"
        v-contextmenu="
          (context: PopoverContext): PopoverInfo => ({
            kind: 'menu',
            placement: 'bottom-right',
            items: menuActionsLike(FIELD_CONTEXT_ACTIONS, { context: { ...context, triggerNode: field } }),
          })
        "
        class="max-w-[200px] truncate data-[dragging=true]:opacity-50"
        :prepared-connection="preparedConnection"
        :node-ptr="toNodeRefOneOf(field)"
        :draggable="true"
        @dragstart.stop="(e: DragEvent) => startDraggingIfAllowed(e, pkgGraph, field)"
      />
    </li>
    <!-- Add button -->
    <button
      class="h-[28px] px-1 text-gray-400 hover:text-gray-700"
      @click="
        (e) => {
          const button = (e.target as HTMLElement).closest('button')!;
          pushPopover({
            trigger: button,
            reference: button,
            info: {
              component: ViewType.PICKER,
              placement: 'bottom-left',
              offset: 'referenceWidth',
              props: { valueType: makeTypeInfo({ benchType: BenchType.TYPE_INFO }) },
              onApply: (typeInfo: TypeIdentity) => {
                createField(pkgConnection.tx, pkgGraph, {
                  anchor: 'inside',
                  target: block!,
                  field: { ...typeInfo, type: fieldType },
                });
              },
            },
          });
        }
      "
    >
      <i class="fas fa-plus mr-1.5" />
      <span> {{ toCamelName(FieldType, props.fieldType) }} </span>
    </button>
  </ul>
</template>

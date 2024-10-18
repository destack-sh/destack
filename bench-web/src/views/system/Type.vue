<script lang="ts" setup>
import { blockToType } from "@/language/block";
import { RUNNABLE_BLOCK_TYPES, toCamelName, TYPE_BLOCK_TYPES } from "@/language/const";
import { createField, FIELD_CONTEXT_ACTIONS } from "@/language/field";
import { cloneNode, moveNode, onNodeMorphed } from "@/language/node";
import { BlockType, FieldType, NodeType, Orientation, Variant, ViewData, type FieldData } from "@/proto/wire";
import { describeNode, isNode, toNodeRefOneOf, unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import type { ActionContext, ActionMapImplementation } from "@/ui/action";
import {
  startDragging,
  startDraggingIfAllowed,
  useMultiDropZone,
  type DraggedContent,
  type MultiAnchor,
} from "@/ui/drag";
import { menuActionsLike, type PopoverContext, type PopoverInfo } from "@/ui/popover";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import Field from "@/views/system/Field.vue";
import { computed, ref, toRef, type Ref } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; preparedConnection?: PreparedGetConnection } & Partial<
    Pick<ViewData, "name" | "variant" | "nodePtr">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const leftRef = ref<HTMLElement | null>(null);
const rightRef = ref<HTMLElement | null>(null);
const leftFieldRefs: Ref<Record<string, InstanceType<typeof Field> | null>> = ref({});
const rightFieldRefs: Ref<Record<string, InstanceType<typeof Field> | null>> = ref({});

const nodePtr = computed(() => unwrapProtoOneOf(props.nodePtr) as TypedNodeReferenceData<NodeType.BLOCK>);
const { graph: pkgGraph, connection: pkgConnection } = props.preparedConnection ?? useExistingConnection(nodePtr);
const block = pkgGraph.getRef(nodePtr, { ignoreAncestors: props.self == null });
const isFunction = computed(() => block.value != null && RUNNABLE_BLOCK_TYPES.includes(block.value.type));
const shouldHaveFields = computed(() => block.value != null && !isFunction.value);
const fields = pkgGraph.getChildrenRef(block, NodeType.FIELD);
const leftZone = computed(() => {
  if (block.value?.type == BlockType.CLASS) {
    return FieldType.MEMBER;
  } else if (block.value?.type == BlockType.CHOICE) {
    return FieldType.OPTION;
  } else if (isFunction.value) {
    return FieldType.INPUT;
  } else {
    return null;
  }
});
const rightZone = computed(() => {
  if (isFunction.value) {
    return FieldType.OUTPUT;
  } else {
    return null;
  }
});
const leftFields = computed(() => {
  return leftZone.value == null ? fields.value : fields.value.filter((f) => f.type == leftZone.value);
});
const rightFields = computed(() => {
  if (isFunction.value) {
    return fields.value.filter((f) => f.type == FieldType.OUTPUT);
  } else {
    return [];
  }
});

// dragging :TypeDragAndDrop
// NOTE: we have separate drop zones for left/right (for function types)
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
  const side = leftRef.value?.contains(event.target as Node) ? "left" : "right";
  const sideType = side == "left" ? leftZone.value : rightZone.value;
  const target = targetId != null ? pkgGraph.get({ id: targetId }) : null;
  if (isNode(node, NodeType.FIELD)) {
    // move field
    if (target != null) {
      if (!isNode(target, NodeType.FIELD)) throw new Error(`unexpected target node: ${describeNode(target)}`);
      moveNode(pkgConnection.tx, pkgGraph, dragged.node, { anchor, target });
    } else {
      moveNode(pkgConnection.tx, pkgGraph, dragged.node, { anchor: "center", target: block.value! });
    }
    if (node.type != sideType) {
      pkgConnection.tx.update(node, { type: sideType ?? undefined }, { debounce: "tick" });
      onNodeMorphed(pkgConnection.tx, pkgGraph, node);
    }
  } else if (isNode(node, NodeType.BLOCK)) {
    // add field with block type
    const type = blockToType(node);
    const fieldIn = { ...type, zone: sideType! };
    if (target != null) {
      if (!isNode(target, NodeType.FIELD)) throw new Error(`unexpected target node: ${describeNode(target)}`);
      createField(pkgConnection.tx, pkgGraph, { field: fieldIn, anchor, target });
    } else {
      createField(pkgConnection.tx, pkgGraph, { field: fieldIn, anchor: "inside", target: block.value! });
    }
  }
}
const { activeDropZone: activeLeftDropZone } = useMultiDropZone({
  name: "type.left",
  container: leftRef,
  targets: leftFieldRefs,
  orientation: Orientation.VERTICAL,
  kinds: ["node"],
  metatypes: [NodeType.BLOCK, NodeType.FIELD],
  fallbackToClosest: true,
  allowDrop,
  onDrop,
});
const { activeDropZone: activeRightDropZone } = useMultiDropZone({
  name: "type.right",
  container: rightRef,
  targets: rightFieldRefs,
  orientation: Orientation.VERTICAL,
  kinds: ["node"],
  metatypes: [NodeType.BLOCK, NodeType.FIELD],
  fallbackToClosest: true,
  allowDrop,
  onDrop,
});
const activeDropZone = computed(() => activeLeftDropZone.value ?? activeRightDropZone.value);
const activeDropZoneSide = computed(() => {
  if (activeLeftDropZone.value != null) return "left";
  else if (activeRightDropZone.value != null) return "right";
  else return null;
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
    createField(pkgConnection.tx, pkgGraph, { anchor: "before", target: field });
  },
  "common.create.below": (action, ctx) => {
    const { field } = getFieldFromContext(ctx);
    if (field == null) return false;
    createField(pkgConnection.tx, pkgGraph, { anchor: "after", target: field });
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
  <div class="flex flex-row gap-x-3">
    <!-- 'Side' zone -->
    <template
      v-for="{ side, sideFields, sideFieldRefs } in isFunction
        ? [
            { side: 'left', sideFields: leftFields, sideFieldRefs: leftFieldRefs },
            { side: 'right', sideFields: rightFields, sideFieldRefs: rightFieldRefs },
          ]
        : [{ side: 'left', sideFields: leftFields, sideFieldRefs: leftFieldRefs }]"
      :key="side"
    >
      <!-- Arrow -->
      <div
        v-if="side == 'right' && fields.length > 0"
        class="flex flex-shrink-0 flex-col items-center justify-center px-2"
      >
        <i class="fas fa-arrow-right-long text-lg text-gray-400" />
      </div>
      <!-- Fields in zone -->
      <!-- NOTE :UX: field zone drop outline should be dotted if dragged is not a field
        (since it's not a move, but a sort of 'copy', and that's how we signal it elsewhere) -->
      <ul
        :ref="(ref: any) => (side == 'left' ? (leftRef = ref) : (rightRef = ref))"
        class="relative flex flex-1 flex-col gap-y-1 rounded"
        :class="[
          activeDropZoneSide == side ? 'outline outline-2 outline-primary-900' : '',
          sideFields.length == 0 && shouldHaveFields ? 'min-h-7 justify-center' : '',
        ]"
      >
        <!-- Empty state -->
        <div
          v-if="sideFields.length == 0"
          class="flex flex-row items-center px-1"
          :class="
            sideFields.length == 0 && leftFields.length + rightFields.length > 0
              ? 'my-1' /* extra padding if other side is not empty */
              : ''
          "
        >
          <i class="fas fa-empty-set mr-1.5 text-gray-400" />
          <span class="text-gray-500">No {{ toCamelName(FieldType, side == "left" ? leftZone : rightZone) }}s</span>
        </div>
        <!-- Drop indicator -->
        <div v-else-if="activeDropZoneSide == side" class="absolute right-1 top-1 text-primary-900">
          {{ toCamelName(FieldType, side == "left" ? leftZone : rightZone) }}
        </div>
        <!-- Field wrapper -->
        <li v-for="(field, i) in sideFields" :key="field.id" class="relative w-fit max-w-[200px]">
          <!-- Drop indicator -->
          <div
            v-if="activeDropZone?.targetId == field.id"
            class="absolute z-10 h-1 w-full rounded-sm bg-primary-900"
            :class="[activeDropZone?.anchor == 'start' ? (i == 0 ? 'top-0' : '-top-[4px]') : '-bottom-[3px]']"
          />
          <!-- Field -->
          <Field
            :ref="(ref: any) => (ref != null ? (sideFieldRefs[field.id] = ref) : delete sideFieldRefs[field.id])"
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
      </ul>
    </template>
  </div>
</template>

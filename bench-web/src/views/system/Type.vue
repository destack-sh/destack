<script lang="ts" setup>
import { BlockType, FieldZone, NodeType, Orientation, Variant, ViewData, type FieldData } from "@/proto/wire";
import {
  describeNode,
  isNode,
  toNodeRef,
  toNodeRefOneOf,
  unwrapProtoOneOf,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import type { ActionContext, ActionMapImplementation } from "@/ui/action";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { cloneNode, createField, moveNode, onNodeMorphed } from "@/language/node";
import { canvas } from "@/system/space";
import { startDragging, useMultiDropZone, type DraggedContent, type MultiAnchor } from "@/ui/drag";
import { menuActionsLike, type PopoverContext, type PopoverInfo } from "@/ui/popover";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import Field from "@/views/system/Field.vue";
import { computed, ref, toRef, type Ref } from "vue";
import { RUNNABLE_BLOCK_TYPES, toCamelName, TYPE_BLOCK_TYPES } from "@/language/const";
import { blockToType } from "@/language/block";

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
    return FieldZone.MEMBER;
  } else if (block.value?.type == BlockType.CHOICE) {
    return FieldZone.OPTION;
  } else if (isFunction.value) {
    return FieldZone.INPUT;
  } else {
    return null;
  }
});
const rightZone = computed(() => {
  if (isFunction.value) {
    return FieldZone.OUTPUT;
  } else {
    return null;
  }
});
const leftFields = computed(() => {
  return leftZone.value == null ? fields.value : fields.value.filter((f) => f.zone == leftZone.value);
});
const rightFields = computed(() => {
  if (isFunction.value) {
    return fields.value.filter((f) => f.zone == FieldZone.OUTPUT);
  } else {
    return [];
  }
});

// dragging
// NOTE: we have separate drop zones for left/right (for function types)
function allowDrop(dragged: DraggedContent, anchor: MultiAnchor, targetId: string | null, event?: DragEvent): boolean {
  if (dragged.kind != "node") return false;
  const node = pkgGraph.get(dragged.node);
  if (isNode(node, NodeType.FIELD) && (node.zone == FieldZone.OPTION) == (block.value?.type == BlockType.CHOICE)) {
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
  if (dragged.kind == "node") {
    const node = pkgGraph.getOrError(dragged.node);
    const side = leftRef.value?.contains(event.target as Node) ? "left" : "right";
    const sideZone = side == "left" ? leftZone.value : rightZone.value;
    const target = targetId != null ? pkgGraph.get({ id: targetId }) : null;
    if (isNode(node, NodeType.FIELD)) {
      // move field here
      if (target != null) {
        if (!isNode(target, NodeType.FIELD)) throw new Error(`unexpected target node: ${describeNode(target)}`);
        moveNode(pkgConnection.tx, pkgGraph, dragged.node, { anchor, target });
        if (node.zone != target.zone) {
          pkgConnection.tx.update(node, { zone: target.zone });
          onNodeMorphed(pkgConnection.tx, pkgGraph, node);
        }
      } else {
        moveNode(pkgConnection.tx, pkgGraph, dragged.node, { anchor: "center", target: block.value! });
        if (node.zone != sideZone) {
          pkgConnection.tx.update(node, { zone: sideZone ?? undefined }, { debounce: "tick" });
          onNodeMorphed(pkgConnection.tx, pkgGraph, node);
        }
      }
    } else if (isNode(node, NodeType.BLOCK)) {
      // add field here
      const type = blockToType(node);
      const fieldIn = { ...type, zone: sideZone! };
      if (target != null) {
        if (!isNode(target, NodeType.FIELD)) throw new Error(`unexpected target node: ${describeNode(target)}`);
        createField(pkgConnection.tx, pkgGraph, {
          field: fieldIn,
          anchor: anchor == "start" ? "before" : "after",
          target,
        });
      } else {
        createField(pkgConnection.tx, pkgGraph, { field: fieldIn, anchor: "inside", target: block.value! });
      }
    }
  }
}
const { activeDropZone: activeLeftDropZone } = useMultiDropZone({
  name: "class",
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
  name: "class",
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
  "common.create.above": {
    action: (action, ctx) => {
      const { field } = getFieldFromContext(ctx);
      if (field == null) return false;
      createField(pkgConnection.tx, pkgGraph, { anchor: "before", target: field });
    },
  },
  "common.create.below": {
    action: (action, ctx) => {
      const { field } = getFieldFromContext(ctx);
      if (field == null) return false;
      createField(pkgConnection.tx, pkgGraph, { anchor: "after", target: field });
    },
  },
  "common.edit.duplicate": {
    action: (action, ctx) => {
      const { field } = getFieldFromContext(ctx);
      if (field == null) return false;
      const duplicate = cloneNode(pkgConnection.tx, pkgGraph, field, { includeChildren: true });
    },
  },
  "common.edit.delete": {
    action: (action, ctx) => {
      const { field } = getFieldFromContext(ctx);
      if (field == null) return false;
      pkgConnection.tx.delete(field);
    },
  },
  "common.edit.archive": {
    action: (action, ctx) => {
      const { field } = getFieldFromContext(ctx);
      if (field == null) return false;
      pkgConnection.tx.archive(field);
    },
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
          <span class="text-gray-500">No {{ toCamelName(FieldZone, side == "left" ? leftZone : rightZone) }}s</span>
        </div>
        <!-- Drop indicator -->
        <div v-else-if="activeDropZoneSide == side" class="absolute right-1 top-1 text-primary-900">
          {{ toCamelName(FieldZone, side == "left" ? leftZone : rightZone) }}
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
                items: menuActionsLike(
                  [
                    'common.edit.rename',
                    'common.edit.morph',
                    'common.edit.duplicate',
                    'common.edit.archive',
                    'common.edit.delete',
                    'common.create.above',
                    'common.create.below',
                    'type*',
                  ],
                  {
                    context: { ...context, triggerNode: field },
                  },
                ),
              })
            "
            role="listitem"
            class="max-w-[200px] truncate data-[dragging=true]:opacity-50"
            :prepared-connection="preparedConnection"
            :node-ptr="toNodeRefOneOf(field)"
            :draggable="true"
            :variant="Variant.STEALTH"
            @dragstart.stop="(e: DragEvent) => startDragging(e, pkgGraph, field)"
          />
        </li>
      </ul>
    </template>
  </div>
</template>

<script lang="ts" setup>
import { BlockType, FieldZone, NodeType, Orientation, Variant, ViewData, type FieldData } from "@/proto/wire";
import { isNode, toNodeReference, type TypedNodeReferenceData } from "@/proto/wiring";
import type { ActionContext, ActionMapImplementation } from "@/system/action";
import { useGetConnection, type PreparedGetConnection } from "@/system/connection";
import { RUNNABLE_BLOCK_TYPES, createField, moveNode, onNodeMorphed, toCamelName } from "@/system/lang";
import { canvas } from "@/system/space";
import { startDragging, useMultiDropZone, type DraggedData, type MultiAnchor } from "@/utils/drag";
import { menuActionsLike, type PopoverInfo } from "@/utils/menu";
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

const nodePtr = toRef(props, "nodePtr") as Ref<TypedNodeReferenceData<NodeType.BLOCK>>;
const { graph: pkgGraph, connection: pkgConnection } =
  props.preparedConnection ??
  useGetConnection(
    { name: `class.${nodePtr.value.id}` },
    computed(() => ({ roots: [nodePtr.value], isEnabled: nodePtr.value != null })),
  );
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
function allowDrop(dragged: DraggedData, anchor: MultiAnchor, targetId: string | null, event?: DragEvent): boolean {
  if (dragged.kind != "node") return false;
  const node = pkgGraph.get(dragged.node);
  if (!isNode(node, NodeType.FIELD)) return false;
  if ((node.zone == FieldZone.OPTION) != (block.value?.type == BlockType.CHOICE)) return false;
  return true;
}
function onDrop(dragged: DraggedData, anchor: MultiAnchor, targetId: string | null, event: DragEvent) {
  if (dragged.kind == "node") {
    const node = pkgGraph.getOrError(dragged.node) as FieldData;
    const side = leftRef.value?.contains(event.target as Node) ? "left" : "right";
    const sideZone = side == "left" ? leftZone.value : rightZone.value;
    if (targetId != null) {
      const target = pkgGraph.getOrError({ id: targetId }) as FieldData;
      moveNode(pkgConnection.tx, pkgGraph, dragged.node, anchor, target);
      if (node.zone != target.zone) {
        pkgConnection.tx.update(node, { zone: target.zone });
        onNodeMorphed(pkgConnection.tx, pkgGraph, node);
      }
    } else {
      moveNode(pkgConnection.tx, pkgGraph, dragged.node, "center", block.value!);
      if (node.zone != sideZone) {
        pkgConnection.tx.updateDebounced(node, { zone: sideZone ?? undefined });
        onNodeMorphed(pkgConnection.tx, pkgGraph, node);
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
  metatypes: [NodeType.FIELD],
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
  metatypes: [NodeType.FIELD],
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
  // TODO :Incomplete: Type fallback to focused/inspection/...? like in other actions?
  return { field };
};
// TODO :Incomplete: Type.actions (move, navigate, ...)
const actions: Partial<ActionMapImplementation<"common">> = {
  // common
  "common.create.above": {
    action: (action, ctx) => {
      const { field } = getFieldFromContext(ctx);
      if (field == null) return false;
      createField(pkgConnection.tx, pkgGraph, "before", field);
    },
  },
  "common.create.below": {
    action: (action, ctx) => {
      const { field } = getFieldFromContext(ctx);
      if (field == null) return false;
      createField(pkgConnection.tx, pkgGraph, "after", field);
    },
  },
  "common.edit.delete": {
    action: (action, ctx) => {
      const { field } = getFieldFromContext(ctx);
      if (field == null) return false;
      pkgConnection.tx.softDelete(field);
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
      <ul
        :ref="(ref: any) => (side == 'left' ? (leftRef = ref) : (rightRef = ref))"
        class="relative flex flex-1 flex-col gap-y-1 rounded"
        :class="[
          activeDropZoneSide == side ? 'outline-dotted outline-2 outline-primary-900' : '',
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
        <div v-else-if="activeDropZoneSide == side" class="absolute right-1 top-1 text-gray-400">
          {{ toCamelName(FieldZone, side == "left" ? leftZone : rightZone) }}
        </div>
        <!-- Field wrapper -->
        <li v-for="(field, i) in sideFields" :key="field.id" class="relative w-fit max-w-[200px]">
          <!-- Drop indicator -->
          <div
            v-if="activeDropZone?.targetId == field.id"
            class="absolute z-10 h-1 w-full rounded-sm bg-primary-400"
            :class="[activeDropZone?.anchor == 'start' ? (i == 0 ? 'top-0' : '-top-[4px]') : '-bottom-[3px]']"
          />
          <!-- Field -->
          <Field
            :ref="(ref: any) => (ref != null ? (sideFieldRefs[field.id] = ref) : delete sideFieldRefs[field.id])"
            v-contextmenu="
              (): PopoverInfo => ({
                kind: 'menu',
                placement: 'bottom-right',
                items: menuActionsLike(
                  [
                    'common.edit.rename',
                    'common.edit.morph',
                    'common.edit.duplicate',
                    'common.edit.delete',
                    'common.create.above',
                    'common.create.below',
                    'type*',
                  ],
                  {
                    context: { triggerNode: field },
                  },
                ),
              })
            "
            role="listitem"
            class="max-w-[200px] truncate data-[dragging=true]:opacity-50"
            :prepared-connection="preparedConnection"
            :node-ptr="toNodeReference(field)"
            :draggable="true"
            :variant="Variant.STEALTH"
            @dragstart.stop="(e: DragEvent) => startDragging(e, pkgGraph, field)"
          />
        </li>
      </ul>
    </template>
  </div>
</template>

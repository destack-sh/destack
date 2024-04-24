<script lang="ts" setup>
import { BlockType, FieldKind, NodeType, Orientation, Variant, ViewData, type FieldData } from "@/proto/wire";
import { describeNode, isNode, toNodeReference, type TypedNodeReferenceData } from "@/proto/wiring";
import type { ActionContext, ActionMapImplementation } from "@/system/action";
import { useGetConnection, type PreparedGetConnection } from "@/system/connection";
import { moveNode } from "@/system/graph";
import { createField, toCamelName } from "@/system/lang";
import { canvas } from "@/system/space";
import { startDragging, useMultiDropZone, type Dragged, type DraggedData, type MultiAnchor } from "@/utils/drag";
import { menuActionsLike, type OverlayMenuInfo } from "@/utils/menu";
import { makeViewId } from "@/views";
import { viewEmits, type ViewExposed } from "@/views/common";
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
const isFunction = computed(() => block.value?.type != BlockType.CLASS);
const fields = pkgGraph.getChildrenRef(block, NodeType.FIELD);
const leftKind = computed(() => {
  if (block.value?.type == BlockType.CLASS) {
    return FieldKind.MEMBER;
  } else if (block.value?.type == BlockType.CHOICE) {
    return FieldKind.OPTION;
  } else if (isFunction.value) {
    return FieldKind.INPUT;
  } else {
    return null;
  }
});
const rightKind = computed(() => {
  if (isFunction.value) {
    return FieldKind.OUTPUT;
  } else {
    return null;
  }
});
const leftFields = computed(() => {
  return leftKind.value == null ? fields.value : fields.value.filter((f) => f.kind == leftKind.value);
});
const rightFields = computed(() => {
  if (isFunction.value) {
    return fields.value.filter((f) => f.kind == FieldKind.OUTPUT);
  } else {
    return [];
  }
});

// dragging
function allowDrop(dragged: DraggedData, anchor: MultiAnchor, targetId: string | null, event?: DragEvent): boolean {
  if (dragged.kind != "node") return false;
  const node = pkgGraph.get(dragged.node);
  if (!isNode(node, NodeType.FIELD)) return false;
  if ((node.kind == FieldKind.OPTION) != (block.value?.type == BlockType.CHOICE)) return false;
  return true;
}
function onDrop(dragged: DraggedData, anchor: MultiAnchor, targetId: string | null, event: DragEvent) {
  if (dragged.kind == "node") {
    const node = pkgGraph.getOrError(dragged.node) as FieldData;
    const side = leftRef.value?.contains(event.target as Node) ? "left" : "right";
    const sideKind = side == "left" ? leftKind.value : rightKind.value;
    if (targetId != null) {
      const target = pkgGraph.getOrError({ id: targetId }) as FieldData;
      moveNode(pkgConnection.tx, pkgGraph, dragged.node, anchor, target);
      if (node.kind != target.kind) {
        pkgConnection.tx.update(node, { kind: target.kind });
      }
    } else {
      moveNode(pkgConnection.tx, pkgGraph, dragged.node, "center", block.value!);
      if (node.kind != sideKind) {
        pkgConnection.tx.updateDebounced(node, { kind: sideKind ?? undefined });
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
  let field = fields.value.find((f) => f.id == ctx?.triggerNode?.id) ?? null;
  // nocheckin: Class.getFieldFromContext from focused?
  return { field };
};
const actions: Partial<ActionMapImplementation<"common">> = {
  // common
  "common.edit.delete": {
    action: (action, ctx) => {
      const { field } = getFieldFromContext(ctx);
      if (field == null) return false;
      pkgConnection.tx.softDelete(field);
    },
  },
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
};

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, actions });
</script>
<template>
  <div class="flex flex-row gap-x-3" :class="[fields.length > 0 ? 'py-1' : '']">
    <!-- 'Side' zone -->
    <template v-for="side in isFunction ? ['left', 'right'] : ['left']" :key="side">
      <!-- Arrow -->
      <div v-if="side == 'right' && fields.length > 0" class="flex flex-col items-center justify-center">
        <i class="fas fa-arrow-right-long text-xl text-gray-700" />
      </div>
      <!-- Fields in zone -->
      <ul
        :ref="(ref: any) => (side == 'left' ? (leftRef = ref) : (rightRef = ref))"
        class="relative flex flex-1 flex-col gap-y-1"
        :class="[activeDropZoneSide == side ? 'rounded outline-dotted outline-2 outline-primary-900' : '']"
      >
        <!-- Drop indicator -->
        <div v-if="activeDropZoneSide == side" class="absolute right-1 top-1 text-gray-400">
          {{ toCamelName(FieldKind, side == "left" ? leftKind : rightKind) }}
        </div>
        <!-- Field wrapper -->
        <li
          v-for="(field, i) in side == 'left' ? leftFields : rightFields"
          :key="field.id"
          class="relative w-fit max-w-[200px]"
        >
          <!-- Drop indicator -->
          <div
            v-if="activeDropZone?.targetId == field.id"
            class="absolute z-10 h-1 w-full rounded-sm bg-primary-400"
            :class="[activeDropZone?.anchor == 'start' ? (i == 0 ? 'top-0' : '-top-[4px]') : '-bottom-[3px]']"
          />
          <!-- Field -->
          <Field
            :ref="
              (ref: any) => {
                const refs = side == 'left' ? leftFieldRefs : rightFieldRefs;
                if (ref == null) delete refs[field.id];
                else refs[field.id] = ref;
              }
            "
            role="listitem"
            class="max-w-full"
            :prepared-connection="preparedConnection"
            :node-ptr="toNodeReference(field)"
            :draggable="true"
            @dragstart.stop="(e: DragEvent) => startDragging(e, pkgGraph, field)"
            :variant="Variant.STEALTH"
            v-contextmenu="
              (): OverlayMenuInfo => ({
                kind: 'menu',
                placement: 'bottom-right',
                items: menuActionsLike(
                  [
                    'common.edit.rename',
                    'common.edit.morph',
                    'common.edit.copy',
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
          />
        </li>
      </ul>
    </template>
  </div>
</template>

<script lang="ts" setup>
import { BlockType, NodeType, Orientation, ViewData, type FieldData } from "@/proto/wire";
import { toNodeReference, type TypedNodeReferenceData } from "@/proto/wiring";
import { useGetConnection, type PreparedGetConnection } from "@/system/connection";
import { moveNode } from "@/system/graph";
import { canvas } from "@/system/space";
import { startDragging, useMultiDropZone } from "@/utils/drag";
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
const fieldRefs: Ref<Record<string, InstanceType<typeof Field> | null>> = ref({});

const nodePtr = toRef(props, "nodePtr") as Ref<TypedNodeReferenceData<NodeType.BLOCK>>;
const { graph: pkgGraph, connection: pkgConnection } =
  props.preparedConnection ??
  useGetConnection(
    { name: `class.${nodePtr.value.id}` },
    computed(() => ({ roots: [nodePtr.value], isEnabled: nodePtr.value != null })),
  );
const block = pkgGraph.getRef(nodePtr, { ignoreAncestors: props.self == null });
const fields = pkgGraph.getChildrenRef(block, NodeType.FIELD);
const isFunction = computed(() => block.value?.type != BlockType.CLASS);

// dragging
const { activeDropZone } = useMultiDropZone({
  name: "class",
  container: leftRef,
  targets: fieldRefs,
  orientation: Orientation.VERTICAL,
  kinds: ["node"],
  metatypes: [NodeType.FIELD],
  fallbackToClosest: true,
  onDrop: (dragged, anchor, targetId) => {
    if (targetId != null && dragged.kind == "node") {
      const target = pkgGraph.getOrError({ id: targetId });
      moveNode(pkgConnection.tx, pkgGraph, dragged.node, anchor, target);
    }
  },
});

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <ul ref="leftRef" class="flex flex-col gap-y-0.5" :class="[fields.length > 0 ? 'py-1' : '']">
    <!-- Field wrapper -->
    <div v-for="(field, i) in fields" :key="field.id" class="w-fit relative">
      <!-- Drop indicator -->
      <div
        v-if="activeDropZone?.targetId == field.id"
        class="absolute w-full z-10 h-1 rounded-sm bg-primary-400"
        :class="[activeDropZone?.anchor == 'start' ? (i == 0 ? 'top-0' : '-top-[4px]') : '-bottom-[3px]']"
      />
      <!-- Field -->
      <Field
        :ref="(ref: any) => (ref != null ? (fieldRefs[field.id] = ref) : delete fieldRefs[field.id])"
        role="listitem"
        :prepared-connection="preparedConnection"
        :node-ptr="toNodeReference(field)"
        :draggable="true"
        @dragstart.stop="(e: DragEvent) => startDragging(e, pkgGraph, field)"
      />
    </div>
  </ul>
</template>

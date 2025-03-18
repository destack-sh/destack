<script lang="ts" setup>
import { toCamelName } from "@/language/core/const";
import { packSubnode } from "@/language/core/node";
import { makeType, TypeIdentity } from "@/language/core/type";
import { createField, useFieldList } from "@/language/source/field";
import {
  BenchType,
  FieldType,
  NodeType,
  Orientation,
  PickerVariant,
  TypeBaseNodeData,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { FIELD_CONTEXT_ACTIONS, type ActionMapKit } from "@/ui/action";
import {
  isDragging,
  startDraggingIfAllowed,
  startSelectingIfAllowed,
  useMultiDropZone,
  useSelectionZone,
} from "@/ui/drag";
import { useNodeListActions } from "@/ui/list";
import { pushPopover } from "@/ui/popover";
import { typeIndex } from "@/ui/search";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import Field from "@/views/nodes/Field.vue";
import SelectionOverlay from "@/views/overlays/SelectionOverlay.vue";
import { computed, ref, toRef, type Ref } from "vue";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    preparedConnection?: PreparedGetConnection;
    fieldType: FieldType;
  } & Partial<Pick<ViewData, "isMinimal" | "orientation" | "nodePtr">>
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const orientation = computed(() => props.orientation ?? Orientation.HORIZONTAL);
const isHorizontal = computed(
  () => orientation.value == Orientation.HORIZONTAL || orientation.value == Orientation.HORIZONTAL_REVERSED,
);
const state = canvas.registerView(self, id);

const containerRef = ref<HTMLElement | null>(null);
const fieldRefs: Ref<Record<string, InstanceType<typeof Field> | null>> = ref({});

const basePtr = computed(() => props.nodePtr);
const { graph, connection } = props.preparedConnection ?? useExistingConnection(basePtr);
const base = graph.getRef(basePtr, { ignoreAncestors: props.self == null }) as Ref<TypeBaseNodeData | null>;
const allFields = graph.getChildrenRef(base, NodeType.FIELD);
const fields = computed(() => allFields.value.filter((f) => f.type == props.fieldType));

// dragging :TypeDragAndDrop
const { allowDrop, onDrop } = useFieldList({
  graph,
  txFactory: () => connection.tx,
  fieldType: toRef(props, "fieldType"),
  base: base,
});
const { activeDropZone } = useMultiDropZone({
  name: "type",
  container: containerRef,
  targetsInOrder: computed(() => fields.value.map((field) => field.id)),
  targetsById: fieldRefs,
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
const actions: Partial<ActionMapKit<"list">> = {
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
        target: node ?? base.value!,
        field: { type: props.fieldType },
      }),
  }),
};

defineExpose<ViewExpose>({ self, id, actions });
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
      class="flex gap-x-2 rounded"
      :class="[isHorizontal ? 'flex-row' : 'flex-col']"
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
          class="h-[30px] cursor-pointer truncate px-1.5 transition-colors duration-150"
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
      <!-- Create -->
      <button
        class="h-[30px] rounded px-2.5 text-left text-gray-400 transition-colors duration-150 hover:bg-gray-100 hover:text-gray-700"
        @click="
          (e) => {
            const button = (e.target as HTMLElement).closest('button')!;
            pushPopover({
              kind: 'view',
              trigger: button,
              reference: button,
              title: `Add ${toCamelName(FieldType, fieldType)}`,
              component: ViewType.PICKER,
              placement: 'bottom-left',
              offset: 'referenceWidth',
              props: {
                valueType: makeType({ benchType: BenchType.TYPE }),
                subnodePacked: packSubnode(NodeType.VIEW, ViewType.PICKER, { variant: PickerVariant.DROPDOWN_LARGE }),
                // @ts-expect-error index is only for Picker props
                index: typeIndex({ id: 'type', graph }),
              },
              onApply: (typeInfo: TypeIdentity) => {
                createField(connection.tx, graph, {
                  anchor: 'inside',
                  target: base!,
                  field: { ...typeInfo, metatype: undefined, icon: undefined, type: fieldType },
                });
              },
            });
          }
        "
      >
        <i class="fas fa-plus mr-1.5" />
        <span> {{ toCamelName(FieldType, props.fieldType) }} </span>
      </button>
    </ul>
    <!-- Selection overlay -->
    <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
  </div>
</template>

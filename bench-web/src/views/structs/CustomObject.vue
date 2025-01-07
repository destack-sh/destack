<script lang="ts" setup>
import { toCamelName } from "@/language/const";
import { useComputedValues } from "@/language/expression";
import { createField, getStorageKey, useFieldList } from "@/language/field";
import { packValue, unpackValue } from "@/language/value";
import {
  ComputedValueData,
  FieldData,
  FieldType,
  NodeType,
  Orientation,
  PathData,
  RectangleData,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { ActionMapImplementation } from "@/ui/action";
import { onAddFieldAction } from "@/ui/detail";
import { startSelectingIfAllowed, useMultiDropZone, useSelectionZone } from "@/ui/drag";
import { useNodeListActions } from "@/ui/list";
import { FULL_WIDTH_VIEW_TYPES, getViewForType } from "@/ui/view";
import SelectionOverlay from "@/views/builtins/SelectionOverlay.vue";
import { ModelValueOptions, viewEmits, type ViewExposed } from "@/views/common";
import Field from "@/views/nodes/Field.vue";
import { getViewComponent, hasViewComponent } from "@/views/registry";
import { computed, ref, toRef, type Ref } from "vue";

const MIN_WIDTH = 320;
const DEFAULT_WIDTH = 280;
const ROW_HEIGHT_MIN = 28;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    size?: Partial<Pick<RectangleData, "width" | "height">>;
    preparedConnection?: PreparedGetConnection;
    isComputable?: boolean;
    computedPrefix?: PathData;
  } & Partial<
    Pick<
      ViewData,
      "name" | "title" | "icon" | "valueType" | "isInput" | "isInline" | "isDisabled" | "isMinimal" | "orientation"
    >
  >
>();
const modelValue = defineModel<any>("modelValue");
const computedValues = defineModel<ComputedValueData[] | undefined>("computedValues");
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const orientation = computed(() => props.orientation ?? Orientation.VERTICAL);
const isHorizontal = computed(
  () => orientation.value == Orientation.HORIZONTAL || orientation.value == Orientation.HORIZONTAL_REVERSED,
);
const buttonRef: Ref<HTMLButtonElement | null> = ref(null);
const width = computed(() => (props.isMinimal ? null : Math.max(MIN_WIDTH, props.size?.width ?? DEFAULT_WIDTH)));
const containerRef = ref<HTMLElement | null>(null);
const fieldRefs: Ref<Record<string, InstanceType<typeof Field> | null>> = ref({});

const basePtr = computed(
  () => props.valueType?.baseTypePtr as TypedNodeReferenceData<NodeType.BLOCK | NodeType.ACTION> | undefined,
);
const { graph, connection } = props.preparedConnection ?? useExistingConnection(basePtr);
const base = graph.getRef(basePtr);
const baseFields = graph.getChildrenRef(base, NodeType.FIELD);
const fields = computed(() =>
  props.valueType?.baseFieldType != null
    ? baseFields.value.filter((f) => f.type == props.valueType!.baseFieldType)
    : baseFields.value,
);
const computer = useComputedValues({
  computedValues: computed(() => computedValues.value ?? []),
  update: (computedValues) => {
    emit("update:computedValues", computedValues);
  },
});

type RowView = {
  field: FieldData;
  storageKey: string;
  viewType?: ViewType;
  viewProps?: any;
  isFullWidth?: boolean;
  computedPath?: PathData;
  computedPathKey?: string;
};
const fieldViews = computed(() => {
  const rows: RowView[] = [];
  for (const field of fields.value) {
    const storageKey = getStorageKey(field, field);
    const view = getViewForType(field);
    const row = {
      field,
      storageKey,
      viewType: view?.type,
      viewProps: { ...view, isInput: props.isInput },
      isFullWidth: FULL_WIDTH_VIEW_TYPES.includes(view?.type!),
    };
    if (props.isComputable) {
      // :RunComputedValue
    }
    rows.push(row);
  }
  return rows;
});

function focus() {
  if (!props.isInline) {
    return buttonRef.value;
  }
}

function apply(value: any) {
  emit("update:modelValue", value);
  emit("apply", value);
}
function clear() {
  apply(props.valueType?.isList ? [] : undefined);
}

// dragging :TypeDragAndDrop
const { allowDrop, onDrop } = useFieldList({
  graph,
  txFactory: () => connection.tx,
  fieldType: computed(() => props.valueType?.baseFieldType ?? FieldType.VARIABLE),
  base,
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
        target: node ?? base.value!,
        field: { type: props.valueType?.baseFieldType },
      }),
  }),
};

defineExpose<ViewExposed>({ self, id, focus, actions });
</script>
<template>
  <ul
    ref="containerRef"
    class="relative flex flex-col gap-y-1.5"
    :class="!isMinimal ? 'py-3' : ''"
    :style="{ width: width != null ? width + 'px' : '100%' }"
    @mousedown="(e) => startSelectingIfAllowed(selectionZone, e)"
  >
    <!-- NOTE :Incomplete: builtin custom object properties :CustomObjectProperties -->
    <!-- Row -->
    <li
      v-for="row of fieldViews"
      :key="row.field.id"
      class="mx-auto w-full"
      :class="[
        row.isFullWidth ? 'flex flex-col gap-y-1' : 'flex flex-row flex-wrap items-center gap-x-1',
        !isMinimal ? 'px-4' : '',
      ]"
      :style="{ minWidth: MIN_WIDTH + 'px', minHeight: ROW_HEIGHT_MIN + 'px' }"
    >
      <!-- Drop indicator -->
      <div
        v-if="activeDropZone?.targetId == row.field.id"
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
      <span class="flex flex-1 flex-row items-center">
        <Field
          :id="row.field.id"
          :ref="(ref: any) => (ref != null ? (fieldRefs[row.field.id] = ref) : delete fieldRefs[row.field.id])"
          is-minimal
          :node-ptr="toNodeRef(row.field)"
        />
        <!-- Actions -->
        <div class="ml-auto pr-1">
          <button
            v-if="isComputable"
            v-tooltip="{ title: `Compute ${row.field.name} dynamically`, small: true, group: 'section.header' }"
            class="rounded px-0.5 transition-colors duration-75"
            :class="
              computer.has(row.computedPathKey)
                ? 'text-primary-700 hover:bg-gray-100'
                : 'text-gray-400 hover/row:bg-gray-100 group-hover/row:text-gray-700'
            "
            @click="() => computer.toggle(row.computedPath!)"
          >
            <i class="fas fa-percent" />
          </button>
        </div>
      </span>
      <!-- Value -->
      <component
        :is="getViewComponent(row.viewType)"
        v-if="
          (modelValue?.[row.storageKey] != null || (isInput && !isDisabled)) &&
          row.viewType != null &&
          hasViewComponent(row.viewType)
        "
        :id="row.field.id + '.value'"
        :class="['ml-auto flex-shrink-0', row.isFullWidth ? '' : 'text-right']"
        :style="{ width: row.isFullWidth ? '100%' : 'calc(90% - 100px)' }"
        v-bind="row.viewProps"
        :model-value="
          unpackValue(modelValue?.[row.storageKey], row.field, {
            graph: graph,
            wrapScalar: false,
            recurseCustomObject: false,
          })
        "
        @update:model-value="
          (value: any) => {
            const valuePacked = packValue(value, row.field, {
              graph: graph,
              wrapScalar: false,
              recurseCustomObject: false,
            });
            const newObject = { ...modelValue, [row.storageKey]: valuePacked };
            const options: ModelValueOptions = { field: row.field, path: [row.storageKey] };
            emit('update:modelValue', newObject, options);
          }
        "
      />
      <div
        v-else-if="row.viewType != null"
        class="ml-auto flex-shrink-0 text-gray-400"
        :class="row.isFullWidth ? '' : 'text-right'"
      >
        <span class="">Unset</span>
      </div>
      <div v-else class="ml-auto flex-shrink-0 text-danger-600" :class="row.isFullWidth ? '' : 'text-right'">
        {{ row.viewType != null ? ViewType[row.viewType] : "No View" }}
      </div>
    </li>
    <!-- Add button -->
    <button
      v-if="!isMinimal || fields.length == 0"
      class="mx-1.5 h-[28px] rounded px-1 text-left text-gray-400 transition-colors duration-150 hover:bg-gray-100 hover:text-gray-700"
      @click="
        (e) => onAddFieldAction(e, valueType?.baseFieldType ?? FieldType.VARIABLE, base!, graph, () => connection.tx)
      "
    >
      <i class="fas fa-plus mr-1.5" />
      <span> {{ valueType?.baseFieldType != null ? toCamelName(FieldType, valueType.baseFieldType) : "Field" }} </span>
    </button>

    <!-- Selection overlay -->
    <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
  </ul>
</template>

<script lang="ts" setup>
import { toCamelName } from "@/language/const";
import { createField, useFieldList } from "@/language/field";
import { getPathKey } from "@/language/path";
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
import { getFieldViews } from "@/ui/view";
import SelectionOverlay from "@/views/builtins/SelectionOverlay.vue";
import { ModelValueOptions, viewEmits, type ViewComponent, type ViewExposed } from "@/views/common";
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
const computedPrefixKey = computed(() => (props.computedPrefix != null ? getPathKey(props.computedPrefix) : undefined));
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
const fieldViews = computed(() => getFieldViews(fields.value, graph, { isInput: props.isInput }));

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
      v-for="{ field, viewType, viewProps, storageKey, isFullWidth } of fieldViews"
      :key="field.id"
      class="mx-auto w-full"
      :class="[
        isFullWidth ? 'flex flex-col gap-y-1' : 'flex flex-row flex-wrap items-center gap-x-[2%]',
        !isMinimal ? 'px-4' : '',
      ]"
      :style="{ minWidth: MIN_WIDTH + 'px', minHeight: ROW_HEIGHT_MIN + 'px' }"
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
      <span class="flex flex-1 flex-row items-center">
        <Field
          :id="field.id"
          :ref="(ref: any) => (ref != null ? (fieldRefs[field.id] = ref) : delete fieldRefs[field.id])"
          is-minimal
          :node-ptr="toNodeRef(field)"
        />
        <!-- Actions -->
        <div class="ml-auto pr-1">
          <button
            v-if="isComputable"
            v-tooltip="{ title: `Compute ${field.name} dynamically`, small: true, group: 'section.header' }"
            class="rounded px-0.5 text-gray-400 transition-colors duration-75 hover:bg-gray-100 hover:text-gray-700"
          >
            <i class="fas fa-percent" />
          </button>
        </div>
      </span>
      <!-- Value -->
      <component
        :is="getViewComponent(viewType)"
        v-if="
          (modelValue?.[storageKey] != null || (isInput && !isDisabled)) &&
          viewType != null &&
          hasViewComponent(viewType)
        "
        :id="field.id + '.value'"
        :class="['ml-auto flex-shrink-0', isFullWidth ? '' : 'text-right']"
        :style="{ width: isFullWidth ? '100%' : 'calc(90% - 100px)' }"
        v-bind="viewProps"
        :model-value="
          unpackValue(modelValue?.[storageKey], field, {
            graph: graph,
            wrapScalar: false,
            recurseCustomObject: false,
          })
        "
        @update:model-value="
          (value: any) => {
            const valuePacked = packValue(value, field, {
              graph: graph,
              wrapScalar: false,
              recurseCustomObject: false,
            });
            const newObject = { ...modelValue, [storageKey]: valuePacked };
            const options: ModelValueOptions = { field, path: [storageKey] };
            emit('update:modelValue', newObject, options);
          }
        "
      />
      <div
        v-else-if="viewType != null"
        class="ml-auto flex-shrink-0 text-gray-400"
        :class="isFullWidth ? '' : 'text-right'"
      >
        <span class="">Unset</span>
      </div>
      <div v-else class="ml-auto flex-shrink-0 text-danger-600" :class="isFullWidth ? '' : 'text-right'">
        {{ viewType != null ? ViewType[viewType] : "No View" }}
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

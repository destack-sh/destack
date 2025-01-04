<script lang="ts" setup>
import { packValue, unpackValue } from "@/language/value";
import { FieldData, NodeType, RectangleData, ViewData, ViewType } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { getFieldViews } from "@/ui/view";
import { ModelValueOptions, viewEmits, type ViewComponent, type ViewExposed } from "@/views/common";
import { getViewComponent, hasViewComponent } from "@/views/registry";
import { computed, ref, toRef, type Ref } from "vue";

const MIN_WIDTH = 320;
const DEFAULT_WIDTH = 280;
const ROW_HEIGHT_MIN = 28;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    modelValue?: any;
    size?: Partial<Pick<RectangleData, "width" | "height">>;
    preparedConnection?: PreparedGetConnection;
  } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "valueType" | "isInput" | "isInline" | "isDisabled" | "isMinimal">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");

const buttonRef: Ref<HTMLButtonElement | null> = ref(null);
const width = computed(() => (props.isMinimal ? null : Math.max(MIN_WIDTH, props.size?.width ?? DEFAULT_WIDTH)));

const basePtr = computed(
  () => props.valueType?.baseTypePtr as TypedNodeReferenceData<NodeType.BLOCK | NodeType.ACTION> | undefined,
);
const { graph } = props.preparedConnection ?? useExistingConnection(basePtr);
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

canvas.registerView(self, id);
defineExpose<ViewExposed & { fields: Ref<FieldData[]> }>({ self, id, focus, fields });
</script>
<template>
  <ul
    class="flex flex-col gap-y-1.5"
    :class="!isMinimal ? 'py-3' : ''"
    :style="{ width: width != null ? width + 'px' : '100%' }"
  >
    <!-- NOTE :Incomplete: builtin custom object properties :CustomObjectProperties -->
    <li
      v-for="fieldView of fieldViews"
      :key="fieldView.field.id"
      class="mx-auto w-full"
      :class="[
        fieldView.isFullWidth ? 'flex flex-col gap-y-1' : 'flex flex-row flex-wrap items-center gap-x-[5%]',
        !isMinimal ? 'px-4' : '',
      ]"
      :style="{ minWidth: MIN_WIDTH + 'px', minHeight: ROW_HEIGHT_MIN + 'px' }"
    >
      <!-- Field -->
      <span class="w-[100px]">
        <span class="max-w-full truncate py-1 text-gray-900">{{ fieldView.field.name }}</span>
      </span>
      <!-- Value -->
      <component
        :is="getViewComponent(fieldView.viewType)"
        v-if="
          (props.modelValue?.[fieldView.storageKey] != null || (isInput && !isDisabled)) &&
          fieldView.viewType != null &&
          hasViewComponent(fieldView.viewType)
        "
        :id="fieldView.field.id + '.value'"
        :class="['ml-auto flex-shrink-0', fieldView.isFullWidth ? '' : 'text-right']"
        :style="{ width: fieldView.isFullWidth ? '100%' : 'calc(90% - 100px)' }"
        v-bind="fieldView.viewProps"
        :model-value="
          unpackValue(props.modelValue?.[fieldView.storageKey], fieldView.field, {
            graph: graph,
            wrapScalar: false,
            recurseCustomObject: false,
          })
        "
        @update:model-value="
          (value: any) => {
            const valuePacked = packValue(value, fieldView.field, {
              graph: graph,
              wrapScalar: false,
              recurseCustomObject: false,
            });
            const newObject = { ...props.modelValue, [fieldView.storageKey]: valuePacked };
            const options: ModelValueOptions = { field: fieldView.field, path: [fieldView.storageKey] };
            emit('update:modelValue', newObject, options);
          }
        "
      />
      <div
        v-else-if="fieldView.viewType != null"
        class="ml-auto flex-shrink-0 text-gray-400"
        :class="fieldView.isFullWidth ? '' : 'text-right'"
      >
        <span class="">Unset</span>
      </div>
      <div v-else class="ml-auto flex-shrink-0 text-danger-600" :class="fieldView.isFullWidth ? '' : 'text-right'">
        {{ fieldView.viewType != null ? ViewType[fieldView.viewType] : "No View" }}
      </div>
    </li>
  </ul>
</template>

<script lang="ts" setup>
import type { Field } from "@/state/module";
import FieldInterface from "@/components/interfaces/FieldInterface.vue";
import ValueInterface from "@/components/interfaces/ValueInterface.vue";
import { useNavigationGrid } from "@/composables/useGrid";
import { onStartTyping } from "@vueuse/core";
import { computed, nextTick, ref, toRef } from "vue";
import { useCurrentModule } from "@/state/module";

type StructAppearance = {
  verticalBorders?: boolean;
  minRowHeight?: number;
  maxRowHeight?: number;
  rowPadding?: number;
  hideFieldOutline?: boolean;
};

const DEFAULT_APPEARANCE = {
  verticalBorders: true,
  minRowHeight: 32, // incl. padding
  maxRowHeight: 220,
  rowPadding: 4,
  hideFieldOutline: true,
};

const props = defineProps<{
  fields: Field[];
  modelValue: Record<string, any>;
  readonly?: boolean;
  readonlyType?: boolean;
  active?: boolean;
  debounced?: boolean;
  fullInputs?: boolean;
  appearance?: StructAppearance;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: Record<string, any>): void;
  (e: "update:field", value: Field): void;
  (e: "duplicate:field", value: Field): void;
  (e: "delete:field", value: Field): void;
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "deleteSelf"): void;
}>();

const module = useCurrentModule();

function readField(field: Field) {
  return props.modelValue[module.getTypedKey(field) as string];
}

function writeField(field: Field, value: any) {
  const key = module.getTypedKey(field);
  if (key == null) return;
  emit("update:modelValue", { ...props.modelValue, [key]: value });
}

function deleteField(key: string) {
  const copy = { ...props.modelValue };
  delete copy[key];
  emit("update:modelValue", copy);
}

const grid = useNavigationGrid<"type" | "value", InstanceType<typeof ValueInterface>>(
  ref(["type", "value"]),
  toRef(props, "fields"),
  {
    gridNavigateUp: () => emit("navigateUp"),
    gridNavigateDown: () => emit("navigateDown"),
  }
);

// auto clear and start editing when typing while not editing
onStartTyping((e) => {
  if (props.readonly) return;
  const cell = grid.findRef((r) => r.$el.parentNode.contains(e.target));
  if (cell != null && cell.rowId != "type") {
    const field = props.fields.find((f) => f.key == cell.column);
    if (field == null) return;
    deleteField(module.getTypedKey(field) as string);
    nextTick(() => cell.ref.edit?.());
  }
});

const appearance = computed(() => ({ ...DEFAULT_APPEARANCE, ...props.appearance }));
const rowHeights = computed(() =>
  props.fields.map((field) =>
    Math.max(
      appearance.value.minRowHeight,
      Math.min(appearance.value.maxRowHeight, grid.getRef(field.id, "value")?.previewSize.height.value ?? 0)
    )
  )
);

defineExpose({
  focus: (position: "first" | "last" | string = "first") => {
    if (position == "first") {
      grid.focus(0, "type");
    } else if (position == "last") {
      grid.focus(-1, "type");
    } else {
      grid.focus(position, "type");
    }
  },
  blur: () => {
    grid.refs.value.forEach((r) => r.blur?.());
  },
});
</script>
<template>
  <table ref="gridRef" class="w-full table-fixed">
    <tr
      v-for="(field, y) in fields"
      :key="field.id"
      :class="[y < fields.length - 1 ? 'border-b border-orange-900 border-opacity-[12%]' : '']"
    >
      <td class="w-1/3 self-start">
        <FieldInterface
          :ref="(el: any) => grid.registerColumnRef(field?.id, 'type', el)"
          :type="field"
          :readonly="(readonly ?? false) || (readonlyType ?? false)"
          orientation="vertical"
          class="w-full self-start border border-transparent p-1 text-gray-400 focus-within:border-solid focus-within:border-orange-900 focus-within:border-opacity-[15%] focus-within:bg-orange-100 hover:bg-orange-100"
          :hide-outline="appearance.hideFieldOutline"
          hide-description
          :model-value="field"
          @update:model-value="emit('update:field', { ...$event, id: field.id, key: field.key } as Field)"
          @delete-self="emit('delete:field', field)"
          @duplicate-self="emit('duplicate:field', field)"
          @navigate-up="grid.navigateUp(field?.id, 'type')"
          @navigate-down="grid.navigateDown(field?.id, 'type')"
          @navigate-right="grid.navigateRight(field?.id, 'type')"
          @navigate-left="grid.navigateLeft(field?.id, 'type')"
          :style="{
            minHeight: appearance.minRowHeight + 'px',
            height: rowHeights[y] + 'px',
          }"
        />
      </td>
      <td class="w-full">
        <ValueInterface
          :ref="(el: any) => grid.registerColumnRef(field.id, 'value', el)"
          :model-value="readField(field)"
          @update:model-value="(val) => writeField(field, val)"
          :type="module.effectiveTypeOf(field)"
          :readonly="readonly ?? false"
          :active="active ?? false"
          :debounced="debounced"
          :supports-drop="false"
          :full="fullInputs ?? false"
          wrap
          @delete-self="deleteField(field.key as string)"
          @navigate-up="grid.navigateUp(field.id, 'value')"
          @navigate-down="grid.navigateDown(field.id, 'value')"
          @navigate-right="grid.navigateRight(field.id, 'value')"
          @navigate-left="grid.navigateLeft(field.id, 'value')"
          class="scroll-hidden h-full w-full max-w-full self-start overflow-auto border border-transparent p-1 focus-within:border-solid focus-within:border-orange-900 focus-within:border-opacity-[15%] focus-within:bg-orange-100 hover:bg-orange-100"
          :class="[appearance.verticalBorders ? 'border-l border-orange-900 border-opacity-[12%]' : '']"
          :style="{ 'max-height': appearance.maxRowHeight + 'px' }"
        />
      </td>
    </tr>
  </table>
</template>

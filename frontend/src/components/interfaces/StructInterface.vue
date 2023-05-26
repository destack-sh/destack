<script lang="ts" setup>
import type { SimpleType } from "@/components/editors/statement";
import TypeTupleInterface from "@/components/interfaces/TypeTupleInterface.vue";
import ValueInterface from "@/components/interfaces/ValueInterface.vue";
import { useNavigationGrid } from "@/composables/useGrid";
import { onStartTyping } from "@vueuse/core";
import { computed, nextTick, ref, toRef } from "vue";

type StructAppearance = {
  verticalBorders?: boolean;
  minRowHeight?: number;
  maxRowHeight?: number;
  rowPadding?: number;
};

const DEFAULT_APPEARANCE = {
  verticalBorders: true,
  minRowHeight: 32, // incl. padding
  maxRowHeight: 220,
  rowPadding: 4,
};

const props = defineProps<{
  fields: SimpleType[];
  modelValue: Record<string, any>;
  readonly?: boolean;
  active?: boolean;
  debounced?: boolean;
  appearance?: StructAppearance;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: Record<string, any>): void;
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "deleteSelf"): void;
}>();

function readField(key: string) {
  return props.modelValue[key];
}

function writeField(key: string, value: any) {
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

onStartTyping((e) => {
  if (props.readonly) return;
  const cell = grid.findRef((r) => r.$el.parentNode.contains(e.target));
  if (cell != null && cell.rowId != "type") {
    const field = props.fields.find((f) => f.key == cell.column);
    deleteField(field?.key as string);
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
</script>
<template>
  <table ref="gridRef" class="table-fixed">
    <tr
      v-for="(field, y) in fields"
      :key="field.id"
      :class="[y < fields.length - 1 ? 'border-b border-orange-900 border-opacity-[12%]' : '']"
    >
      <td class="w-1/4">
        <TypeTupleInterface
          :ref="(el: any) => grid.registerColumnRef(field?.id, 'type', el)"
          :type="field"
          readonly
          orientation="vertical"
          class="w-full self-start border border-transparent px-1 py-0.5 text-gray-400 focus-within:border-solid focus-within:border-orange-900 focus-within:border-opacity-[15%] focus-within:bg-orange-100 hover:bg-orange-100"
          :model-value="field"
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
      <!-- main record should always exist but just in case? -->
      <ValueInterface
        :ref="(el: any) => grid.registerColumnRef(field.id, 'value', el)"
        :model-value="readField(field.key as string)"
        @update:model-value="(val) => writeField(field.key as string, val)"
        :type="field"
        :readonly="readonly"
        :active="active"
        :debounced="debounced"
        :supports-drop="false"
        @delete-self="deleteField(field.key as string)"
        @navigate-up="grid.navigateUp(field.id, 'value')"
        @navigate-down="grid.navigateDown(field.id, 'value')"
        @navigate-right="grid.navigateRight(field.id, 'value')"
        @navigate-left="grid.navigateLeft(field.id, 'value')"
        class="h-full w-full self-start overflow-hidden border border-transparent p-1 focus-within:border-solid focus-within:border-orange-900 focus-within:border-opacity-[15%] focus-within:bg-orange-100 hover:bg-orange-100"
        :class="[appearance.verticalBorders ? 'border-l border-orange-900 border-opacity-[12%]' : '']"
        :style="{ 'max-height': appearance.maxRowHeight + appearance.rowPadding * 2 + 'px' }"
      />
    </tr>
  </table>
</template>

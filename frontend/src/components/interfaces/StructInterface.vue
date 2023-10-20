<script lang="ts" setup>
import type { Field, ResolvedField, Statement } from "@/state/module";
import FieldInterface from "@/components/interfaces/FieldInterface.vue";
import ValueInterface from "@/components/interfaces/ValueInterface.vue";
import { useNavigationGrid } from "@/composables/useGrid";
import { onStartTyping } from "@vueuse/core";
import { computed, nextTick, ref, toRef, type Ref } from "vue";
import { useCurrentModule } from "@/state/module";
import { TypeTag } from "@/gql/graphql";
import { ChevronDoubleDownIcon, DocumentDuplicateIcon, QueueListIcon, TableCellsIcon } from "@heroicons/vue/24/outline";
import { IS_DEBUG } from "@/utils/globals";

type StructAppearance = {
  verticalBorders?: boolean;
  minRowHeight?: number;
  maxRowHeight?: number;
  rowPadding?: number;
  hideFieldOutline?: boolean;
  hideFieldType?: boolean;
  minimalFields?: boolean;
};

const DEFAULT_APPEARANCE = {
  verticalBorders: true,
  minRowHeight: 32, // incl. padding
  maxRowHeight: 220,
  rowPadding: 4,
  hideFieldOutline: true,
  hideFieldType: true,
  minimalFields: false,
};

const props = defineProps<{
  type: Field | Statement | Field[];
  isOutput?: boolean;
  modelValue: Record<string, any>;
  readonly?: boolean;
  readonlyType?: boolean;
  active?: boolean;
  debounced?: boolean;
  fullInputs?: boolean;
  showControls?: boolean;
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

const appearance = computed(() => ({ ...DEFAULT_APPEARANCE, ...props.appearance }));
const module = useCurrentModule();
const fields = computed(() => {
  if (Array.isArray(props.type)) {
    return props.type;
  }
  const statement = props.type.__typename == "Statement" ? props.type : module.statementOf(props.type.referenceCk);
  return (
    statement?.resolvedFields
      ?.map((f) => f as ResolvedField)
      .map((f) => (f?.fieldCk == null ? null : module.fieldOf(f.fieldCk)))
      .filter((f) => f != null && f.deletedAt == null)
      .map((f) => f as Field) ?? []
  );
});
const display: Ref<"tree" | "grid"> = ref("grid");

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

// grid

const grid = useNavigationGrid<"type" | "value", InstanceType<typeof ValueInterface | typeof FieldInterface>>(
  ref(["type", "value"]),
  fields,
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
    const field = fields.value.find((f) => f.key == cell.column);
    if (field == null) return;
    deleteField(module.getTypedKey(field) as string);
    nextTick(() => (cell.ref as InstanceType<typeof ValueInterface>).edit?.());
  }
});

const rowHeights = computed(() =>
  fields.value.map((field) =>
    Math.max(
      appearance.value.minRowHeight,
      Math.min(appearance.value.maxRowHeight, grid.getRef(field.id, "value")?.previewSize.height.value ?? 0)
    )
  )
);

// tree (inline)

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
  openField(fieldId: string) {
    (grid.getRef(fieldId, "type") as InstanceType<typeof FieldInterface>)?.open("all");
  },
  blur: () => {
    grid.refs.value.forEach((r) => r.blur?.());
  },
});
</script>
<template>
  <!-- Wrapper for controls -->
  <div class="relative w-full">
    <!-- Grid view -->
    <table v-if="display == 'grid'" ref="gridRef" class="w-full table-fixed">
      <tr v-for="(field, y) in fields" :key="field.id">
        <td
          class="w-1/3 self-start p-0"
          v-if="!appearance.minimalFields"
          :class="[appearance.verticalBorders && y > 0 ? 'border-t border-red-900/[12%]' : '']"
        >
          <FieldInterface
            :ref="(el: any) => grid.registerColumnRef(field?.id, 'type', el)"
            :type="field"
            :readonly="(readonly ?? false) || (readonlyType ?? false)"
            orientation="vertical"
            class="w-full self-start border-r border-amber-900/[12%] bg-amber-100 px-1.5 py-1 text-gray-400 focus-within:border-solid focus-within:bg-amber-200 hover:bg-amber-200"
            :hide-outline="appearance.hideFieldOutline"
            hide-text
            :ref-types="[TypeTag.Enum, TypeTag.Struct]"
            :model-value="field"
            @update:model-value="emit('update:field', { ...$event, id: field.id, key: field.key } as Field)"
            @delete-self="emit('delete:field', field)"
            @duplicate-self="emit('duplicate:field', field)"
            @navigate-up="grid.navigateUp(field?.id, 'type')"
            @navigate-down="grid.navigateDown(field?.id, 'type')"
            @navigate-right="grid.navigateRight(field?.id, 'type')"
            @navigate-left="grid.navigateLeft(field?.id, 'type')"
            @enter="grid.navigateDown(field?.id, 'type')"
            :style="{
              minHeight: appearance.minRowHeight + 'px',
              height: rowHeights[y] + 'px',
            }"
          />
        </td>
        <td class="w-full p-0" :class="[appearance.verticalBorders && y > 0 ? 'border-t border-orange-900/[12%]' : '']">
          <!-- should probably separate the 'minimal fields' out, but not sure what becomes of that yet -->
          <div class="w-full px-1" v-if="appearance.minimalFields">
            <span class="text-xs font-semibold text-gray-500">{{ field.name }}</span>
          </div>
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
            :style="{ 'max-height': appearance.maxRowHeight + 'px' }"
          />
        </td>
      </tr>
    </table>
    <!-- Tree view -->
    <div v-else class="flex flex-col">
      {{ modelValue /* nocheckin tree */ }}
    </div>
    <!-- Controls (featured flagged for debug) -->
    <div v-if="showControls && IS_DEBUG" class="absolute right-0.5 top-0.5 flex flex-row gap-1 bg-white p-0.5">
      <!-- Expand/collapse all -->
      <button v-if="display == 'tree'" class="rounded-sm p-0.5 text-gray-400 hover:bg-orange-100 hover:text-gray-700">
        <component :is="ChevronDoubleDownIcon" class="h-4 w-4" />
      </button>
      <!-- Toggle view -->
      <button
        @click="display = display == 'tree' ? 'grid' : 'tree'"
        class="rounded-sm p-0.5 text-gray-400 hover:bg-orange-100 hover:text-gray-700"
      >
        <component :is="display == 'grid' ? TableCellsIcon : QueueListIcon" class="h-4 w-4" />
      </button>
      <!-- Copy json -->
      <button class="rounded-sm p-0.5 text-gray-400 hover:bg-orange-100 hover:text-gray-700">
        <DocumentDuplicateIcon class="h-4 w-4" />
      </button>
    </div>
  </div>
</template>

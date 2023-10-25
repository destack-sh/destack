<script lang="ts" setup>
import { useElementRefs } from "@/composables/useGrid";
import { TypeHint, TypeTag, type Field } from "@/gql/graphql";
import { useCurrentModule, TypeFlag, type ResolvedField } from "@/state/module";
import { PlusIcon, RectangleGroupIcon } from "@heroicons/vue/24/solid";
import { computed, ref, watch, type Ref } from "vue";
import StructInterface from "@/components/interfaces/StructInterface.vue";
import { pinAbsoluteElement } from "@/composables/useFixed";
import FadeTransition from "@/components/basic/FadeTransition.vue";

const props = defineProps<{
  type: Field;
  modelValue: Record<string, any>[];
  readonly?: boolean;
  preview?: boolean;
  wrap?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: Record<string, any>[]): void;
  (e: "close"): void;
}>();

const addButtonRef = ref<HTMLButtonElement | null>(null);
const structInlineRefs = useElementRefs();
const activeIndex = ref<number | null>(null);
const editingIndex = ref<boolean>(false);
const structRef = ref<HTMLDivElement | null>(null);
const structOpenPos = ref<{ x: number; y: number } | null>(null);
pinAbsoluteElement(structRef, {
  pos: true,
  sourcePos: structOpenPos,
  keepInView: true,
});

// set open pos when active index changes
watch(activeIndex, () => {
  if (activeIndex.value == null) {
    structOpenPos.value = null;
  } else {
    const rect = structInlineRefs.refs.value[activeIndex.value]?.getBoundingClientRect();
    structOpenPos.value = { x: rect.left, y: rect.top };
  }
});

const module = useCurrentModule();
const isArray = computed(() => Boolean(props.type.flags & TypeFlag.IS_ARRAY));
const runtimeType = computed(() => module.statementOf(props.type.referenceCk));

const fields = computed(
  () =>
    runtimeType.value?.resolvedFields
      ?.map((n) => n as ResolvedField)
      .sort((a, b) => (a.orderKey < b.orderKey ? -1 : 1))
      .map((n) => (n?.fieldCk == null ? null : module.fieldOf(n.fieldCk)))
      .filter((n) => n != null && n.deletedAt == null)
      .map((n) => n as Field) ?? []
);
const titleField: Ref<Field | undefined> = computed(() => {
  // get first name or string field
  const nameField = fields.value.find((f) => f.hint == TypeHint.Name);
  if (nameField) return nameField;
  const stringField = fields.value.find((f) => f.tag == TypeTag.String);
  if (stringField) return stringField;
  return undefined;
});

function open(idx: number) {
  activeIndex.value = idx;
  editingIndex.value = true;
}

function update(idx: number, value: Record<string, any>) {
  emit(
    "update:modelValue",
    props.modelValue.map((v, j) => (j == idx ? value : v))
  );
}

function remove(idx: number) {
  emit(
    "update:modelValue",
    props.modelValue.filter((_, j) => j != idx)
  );
}

function focus() {
  addButtonRef.value?.focus();
}

function blur() {
  addButtonRef.value?.blur();
  structInlineRefs.refs.value.forEach((r) => r?.blur());
  activeIndex.value = null;
  editingIndex.value = false;
}

defineExpose({
  focus,
  blur,
});
</script>
<template>
  <div
    class="flex h-full w-full flex-row gap-1"
    :class="[wrap ? 'flex-wrap' : '']"
    @mouseleave="(activeIndex = null), (editingIndex = false)"
  >
    <!-- Inline struct views -->
    <div
      v-for="(struct, i) in modelValue"
      :ref="(el: any) => structInlineRefs.registerRef(i.toString(), el)"
      :key="i"
      class="group/struct relative flex flex-row items-center gap-1 px-1 hover:cursor-pointer"
      @click.stop.prevent="readonly || open(i)"
      @keydown.enter.stop.prevent="readonly || open(i)"
      @focus="editingIndex || (activeIndex = i)"
      @blur="editingIndex || (activeIndex = null)"
      @mouseover="editingIndex || (activeIndex = i)"
    >
      <!-- Struct icon -->
      <RectangleGroupIcon class="h-4 w-4 text-orange-600" />
      <!-- Struct title -->
      <span
        v-if="titleField != null && struct[module.getTypedKey(titleField) ?? ''] != undefined"
        class="min-w-[10px] whitespace-nowrap text-gray-900"
        >{{ struct[module.getTypedKey(titleField) ?? ""] }}</span
      >
      <!-- default to type name if we don't have anything -->
      <span v-else class="text-gray-500 group-hover/struct:text-gray-700">{{ runtimeType?.name }}</span>
      <!-- Delete button -->
      <button
        v-if="!preview"
        class="text-gray-300 focus:text-gray-700 group-hover/struct:text-gray-500"
        @click.stop.prevent="remove(i)"
      >
        x
      </button>
    </div>
    <!-- Struct preview on hover -->
    <!-- TODO @UX: fix struct editability -->
    <!-- pin & fixed here is on purpose, value interfaces are usually clipped so we can't use absolute. -->
    <FadeTransition>
      <div
        v-if="activeIndex != null"
        ref="structRef"
        class="fixed z-20 mt-5 w-96 rounded-sm border bg-white p-1 shadow-md ring-1 ring-orange-900 ring-opacity-[12%]"
      >
        <StructInterface
          :type="type"
          :model-value="modelValue[activeIndex ?? 0]"
          @update:modelValue="update(activeIndex ?? 0, $event)"
          @close="(activeIndex = null), (editingIndex = false)"
          debounced
          show-controls
          :readonly="!editingIndex || true /* TODO @UX: editing structs in struct input fails */"
          :appearance="{ minimalFields: true, hideFieldType: true, view: 'tree' }"
        />
      </div>
    </FadeTransition>
    <!-- Add button -->
    <button
      v-if="!preview && !readonly && (isArray || modelValue.length == 0)"
      ref="addButtonRef"
      class="self-end justify-self-end rounded-sm border-gray-300 px-0.5 transition hover:bg-orange-100 focus:bg-orange-100 focus:outline-none group-focus-within/iface:opacity-100 group-hover/iface:opacity-100"
    >
      <!-- TODO @Feature: edit structs in interface -->
      <PlusIcon class="h-4 w-4 text-gray-400" />
    </button>
    <!-- Empty content for scaling -->
    <template v-if="preview && modelValue.length == 0">&nbsp;</template>
  </div>
</template>

<script lang="ts" setup>
import CreateFieldInterface from "@/components/interfaces/CreateFieldInterface.vue";
import StructInterface from "@/components/interfaces/StructInterface.vue";
import type { StatementEmit, StatementProps } from "@/components/statements";
import { TypeTag, type Field } from "@/gql/graphql";
import type { StatementAction } from "@/state/bench";
import { useOperations } from "@/state/operations";
import { useFields } from "@/state/statement";
import { SquaresPlusIcon, PlusIcon } from "@heroicons/vue/24/outline";
import { useMouseInElement } from "@vueuse/core";
import { computed, nextTick, ref, toRef, type Ref } from "vue";

const props = defineProps<Pick<StatementProps, "statement" | "focused" | "readonly">>();
const emit = defineEmits<StatementEmit>();

const ops = useOperations();
const { allFields, fields, createNewField, duplicateField, updateField, deleteField } = useFields(
  toRef(props, "statement")
);

const structRef: Ref<InstanceType<typeof StructInterface> | null> = ref(null);
const wrapperRef = ref<HTMLDivElement | null>(null);
const addFieldRef: Ref<HTMLButtonElement | null> = ref(null);
const createFieldRef: Ref<InstanceType<typeof CreateFieldInterface> | null> = ref(null);
const position = useMouseInElement(computed(() => wrapperRef.value));

function createNewFieldAndFocus(template: Field) {
  const field = createNewField(template);
  nextTick(() => structRef.value?.openField(field.id));
}

function duplicateFieldAndFocus(field: Pick<Field, "id">) {
  const newField = duplicateField(field.id);
  if (newField == null) return;
  nextTick(() => structRef.value?.focus(newField.id));
}

function writeValue(value: any) {
  ops.symbol.updateValue(null, props.statement.id, props.statement.value, value);
}

const actions = computed(() => {
  const actions: StatementAction[] = [];
  actions.push({
    label: "Add field",
    groupId: "edit",
    icon: SquaresPlusIcon,
    action: () => {
      createFieldRef.value?.show();
    },
  });
  return actions;
});

function focus(position: "first" | "last" = "first") {
  if (position == "first") {
    if (fields.value.length > 0) {
      structRef.value?.focus("first");
    } else {
      addFieldRef.value?.focus();
    }
  } else {
    if (addFieldRef.value != null) {
      addFieldRef.value.focus();
    } else {
      structRef.value?.focus("last");
    }
  }
}

function focusLastField() {
  if (fields.value.length > 0) {
    structRef.value?.focus("last");
  } else {
    focus("first");
  }
}

function focusEnd() {
  if (addFieldRef.value != null) {
    addFieldRef.value.focus();
  } else {
    emit("navigateDown");
  }
}

defineExpose({
  focus,
  blur: () => {
    structRef.value?.blur?.();
  },
  // prevent outer drag and drop while inside grid
  capturingDrag: computed(() => !position.isOutside.value),
  actions,
});
</script>
<template>
  <div ref="wrapperRef">
    <!-- Value -->
    <StructInterface
      ref="structRef"
      class="-mx-1 w-full rounded-sm border-y border-orange-900/[12%]"
      :type="allFields"
      :model-value="statement.value ?? {}"
      @update:model-value="writeValue($event)"
      @update:field="updateField($event, $event)"
      @delete:field="deleteField($event)"
      @duplicate:field="duplicateFieldAndFocus($event)"
      @navigate-up="emit('navigateUp')"
      @navigate-down="focusEnd"
      :readonly="readonly"
      :active="focused"
      :appearance="{ hideFieldType: false, minimalFields: false }"
      debounced
    />
    <!-- Add a field -->
    <button
      v-if="!readonly"
      ref="addFieldRef"
      tabindex="-1"
      class="-mx-1 flex w-full select-none flex-row items-center gap-0.5 border-b border-amber-900/[12%] px-0.5 py-1 text-gray-300 outline-none hover:bg-amber-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
      @click="createFieldRef?.show()"
      @enter="createFieldRef?.show()"
      @keydown.up.exact.prevent="focusLastField"
      @keydown.down.exact.prevent="emit('navigateDown')"
    >
      <PlusIcon class="h-4 w-4" /> Field
    </button>
    <CreateFieldInterface
      ref="createFieldRef"
      :title="'New field'"
      :ref-types="[TypeTag.Enum, TypeTag.Struct]"
      @select="(f) => createNewFieldAndFocus(f as Field)"
    />
  </div>
</template>

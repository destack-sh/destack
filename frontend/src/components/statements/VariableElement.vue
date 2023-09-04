<script lang="ts" setup>
import CreateFieldInterface from "@/components/interfaces/CreateFieldInterface.vue";
import StructInterface from "@/components/interfaces/StructInterface.vue";
import StatementActions from "@/components/statements/StatementActions.vue";
import StatementTags from "@/components/statements/StatementTags.vue";
import TypedStatementDeclaration from "@/components/statements/TypedStatementDeclaration.vue";
import type { Field } from "@/gql/graphql";
import type { StatementAction } from "@/state/bench";
import { useOperations } from "@/state/operations";
import { useStatementContext } from "@/state/statement";
import { CubeTransparentIcon, SquaresPlusIcon, PlusIcon, TagIcon } from "@heroicons/vue/24/outline";
import { useMouseInElement } from "@vueuse/core";
import { computed, nextTick, ref, type Ref } from "vue";

const props = defineProps<{ folded?: boolean }>();
const emit = defineEmits<{ (e: "toggleFold"): void; (e: "toggleActions"): void }>();
const context = useStatementContext();
const ops = useOperations();

const declarationRef: Ref<InstanceType<typeof TypedStatementDeclaration> | null> = ref(null);
const tagsRef: Ref<InstanceType<typeof StatementTags> | null> = ref(null);
const structRef: Ref<InstanceType<typeof StructInterface> | null> = ref(null);
const addFieldRef: Ref<HTMLButtonElement | null> = ref(null);
const createFieldRef: Ref<InstanceType<typeof CreateFieldInterface> | null> = ref(null);
const position = useMouseInElement(computed(() => structRef.value?.$el));

function createUnionField() {
  context.createUnionField();
  nextTick(() => declarationRef.value?.focusLastBase());
}

function createNewField(template: Field) {
  const field = context.createNewField(template);
  nextTick(() => structRef.value?.openField(field.id));
}

function duplicateField(field: Pick<Field, "id">) {
  const newField = context.duplicateField(field.id);
  if (newField == null) return;
  nextTick(() => structRef.value?.focus(newField.id));
}

function writeValue(value: any) {
  ops.symbol.updateValue(null, context.statement.value.id, context.statement.value.value, value);
}

function unfoldIfFolded() {
  if (props.folded) emit("toggleFold");
}

const actions = computed(() => {
  const actions: StatementAction[] = [];
  actions.push({
    label: "Extend type",
    icon: CubeTransparentIcon,
    action: () => {
      unfoldIfFolded();
      createUnionField();
    },
    hideInline: true,
  });
  actions.push({
    label: "Add tag",
    icon: TagIcon,
    action: () => {
      unfoldIfFolded();
      tagsRef.value?.open();
    },
  });
  actions.push({
    label: "Add field",
    icon: SquaresPlusIcon,
    action: () => {
      unfoldIfFolded();
      createFieldRef.value?.show();
    },
  });
  return actions;
});
context.setCustomActions(actions);

function focus(position: "first" | "last" = "first") {
  if (position == "first" || props.folded) {
    declarationRef.value?.focus();
  } else {
    if (addFieldRef.value != null) {
      addFieldRef.value.focus();
    } else {
      structRef.value?.focus("last");
    }
  }
}

function focusLastField() {
  if (context.allFields.value.length > 0) {
    structRef.value?.focus("last");
  } else {
    focus("first");
  }
}

function focusEnd() {
  if (addFieldRef.value != null) {
    addFieldRef.value.focus();
  } else {
    context.navigateDown();
  }
}

defineExpose({
  focus,
  blur: () => {
    declarationRef.value?.blur();
    structRef.value?.blur?.();
  },
  // prevent outer drag and drop while inside grid
  innerDrag: computed(() => !position.isOutside.value),
});
</script>
<template>
  <!-- Value -->
  <StructInterface
    ref="structRef"
    class="-mx-1 w-full"
    :fields="context.allFields.value"
    :model-value="context.statement.value.value ?? {}"
    @update:model-value="writeValue($event)"
    @update:field="context.updateField($event, $event)"
    @delete:field="context.deleteField($event)"
    @duplicate:field="duplicateField($event)"
    @navigate-up="declarationRef?.focus"
    @navigate-down="focusEnd"
    :readonly="context.readonly.value"
    :active="context.focused.value"
    :appearance="{ hideFieldType: false, minimalFields: false }"
    debounced
  />
  <!-- Add a field -->
  <button
    v-if="!context.readonly.value"
    ref="addFieldRef"
    tabindex="-1"
    class="flex w-fit select-none flex-row items-center gap-0.5 rounded-sm px-0.5 text-gray-300 outline-none hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
    @click="createFieldRef?.show()"
    @enter="createFieldRef?.show()"
    @keydown.up.exact.prevent="focusLastField"
    @keydown.down.exact.prevent="context.navigateDown"
  >
    <PlusIcon class="h-4 w-4" /> Field
  </button>
  <CreateFieldInterface
    ref="createFieldRef"
    :title="'New field on ' + context.statement.value.name"
    @select="(f) => createNewField(f as Field)"
  />
</template>

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
import { computed, nextTick, ref, type Ref } from "vue";

const props = defineProps<{ folded?: boolean }>();
const emit = defineEmits<{ (e: "toggleFold"): void }>();
const context = useStatementContext();
const ops = useOperations();

const declarationRef: Ref<InstanceType<typeof TypedStatementDeclaration> | null> = ref(null);
const tagsRef: Ref<InstanceType<typeof StatementTags> | null> = ref(null);
const gridRef: Ref<InstanceType<typeof StructInterface> | null> = ref(null);
const addFieldRef: Ref<HTMLButtonElement | null> = ref(null);
const createFieldRef: Ref<InstanceType<typeof CreateFieldInterface> | null> = ref(null);

function createUnionField() {
  context.createUnionField();
  nextTick(() => declarationRef.value?.focusLastBase());
}

function createNewField(template: Field) {
  const field = context.createNewField(template);
  nextTick(() => gridRef.value?.focus(field.id));
}

function duplicateField(field: Pick<Field, "id">) {
  const newField = context.duplicateField(field.id);
  if (newField == null) return;
  nextTick(() => gridRef.value?.focus(newField.id));
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
      gridRef.value?.focus("last");
    }
  }
}

function focusLastField() {
  if (context.allFields.value.length > 0) {
    gridRef.value?.focus("last");
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
    gridRef.value?.blur?.();
  },
});
</script>
<template>
  <div>
    <div class="flex max-w-full flex-row justify-between">
      <div class="flex flex-row">
        <TypedStatementDeclaration
          ref="declarationRef"
          @navigate-up="context.navigateUp"
          @navigate-down="gridRef?.focus"
        />
        <StatementTags ref="tagsRef" class="ml-1.5" />
      </div>
      <div
        class="flex flex-row items-center gap-1 transition duration-150 group-hover/statement:opacity-100"
        :class="context.focused.value ? '' : 'opacity-0'"
      >
        <StatementActions :extra-actions="actions" />
        <CreateFieldInterface
          ref="createFieldRef"
          :title="'New field on ' + context.statement.value.name"
          @select="createNewField"
        />
      </div>
    </div>
    <!-- Folded info -->
    <button
      v-if="folded"
      class="-mx-0.5 flex max-w-full flex-row gap-1.5 truncate px-0.5 text-gray-400 hover:bg-gray-100"
      @click="$emit('toggleFold')"
    >
      <span v-for="field in context.allFields.value" :key="field.id">{{ field.name }}</span>
    </button>
    <!-- Value -->
    <StructInterface
      v-if="!folded"
      ref="gridRef"
      class="-mx-1 w-full table-fixed"
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
      debounced
    />
    <div v-if="!folded" class="mb-1">
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
    </div>
  </div>
</template>

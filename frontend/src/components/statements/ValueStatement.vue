<script lang="ts" setup>
import CreateFieldInterface from "@/components/interfaces/CreateFieldInterface.vue";
import StructInterface from "@/components/interfaces/StructInterface.vue";
import InlineActionsCell from "@/components/statements/InlineActionsCell.vue";
import TypedDeclarationCell from "@/components/statements/TypedDeclarationCell.vue";
import type { Field } from "@/gql/graphql";
import type { StatementAction } from "@/state/bench";
import { useOperations } from "@/state/operations";
import { useStatementContext } from "@/state/statement";
import { CubeTransparentIcon, SquaresPlusIcon, PlusIcon } from "@heroicons/vue/24/outline";
import { computed, nextTick, ref, type Ref } from "vue";

const props = defineProps<{ folded?: boolean }>();
const context = useStatementContext();
const ops = useOperations();

const declarationRef: Ref<InstanceType<typeof TypedDeclarationCell> | null> = ref(null);
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

const actions = computed(() => {
  const actions: StatementAction[] = [];
  actions.push({
    label: "Extend type",
    icon: CubeTransparentIcon,
    action: () => createUnionField(),
    hideInline: true,
  });
  actions.push({
    label: "Add field",
    icon: SquaresPlusIcon,
    action: () => createFieldRef.value?.show(),
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
    <div class="flex flex-row justify-between">
      <div class="flex flex-row">
        <TypedDeclarationCell ref="declarationRef" @navigate-up="context.navigateUp" @navigate-down="gridRef?.focus" />
        <!-- Folded info -->
        <!-- Folded info -->
        <div v-if="folded" class="ml-1 text-gray-400">
          <span>{{ context.allFields.value.length }} fields</span>
        </div>
      </div>
      <div
        class="flex flex-row items-center gap-1 transition duration-150 group-hover/statement:opacity-100"
        :class="context.focused.value ? '' : 'opacity-0'"
      >
        <InlineActionsCell :extra-actions="actions" />
        <CreateFieldInterface
          ref="createFieldRef"
          :title="'New field on ' + context.statement.value.name"
          @select="createNewField"
        />
      </div>
    </div>
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

<script lang="ts" setup>
import StructInterface from "@/components/interfaces/StructInterface.vue";
import TypedDeclarationCell from "@/components/statements/TypedDeclarationCell.vue";
import InlineActionsCell from "@/components/statements/InlineActionsCell.vue";
import type { StatementAction } from "@/state/bench";
import { useStatementContext } from "@/state/statement";
import { CubeTransparentIcon, SquaresPlusIcon } from "@heroicons/vue/24/outline";
import { computed, type Ref, ref, nextTick } from "vue";
import CreateFieldInterface from "@/components/interfaces/CreateFieldInterface.vue";
import { useOperations } from "@/state/operations";
import type { Field } from "@/gql/graphql";

const context = useStatementContext();
const ops = useOperations();

const declarationRef: Ref<InstanceType<typeof TypedDeclarationCell> | null> = ref(null);
const gridRef: Ref<InstanceType<typeof StructInterface> | null> = ref(null);
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

const extraStatementActions = computed(() => {
  const actions: StatementAction[] = [];
  actions.push({
    label: "Extend type",
    icon: CubeTransparentIcon,
    action: () => createUnionField(),
  });
  actions.push({
    label: "Add field",
    icon: SquaresPlusIcon,
    action: () => createFieldRef.value?.show(),
  });
  return actions;
});

defineExpose({
  focus: (position: "first" | "last" = "first") => {
    declarationRef.value?.focus();
  },
  blur: () => {
    declarationRef.value?.blur();
    gridRef.value?.blur?.();
  },
});
</script>
<template>
  <div>
    <div class="flex flex-row justify-between">
      <TypedDeclarationCell ref="declarationRef" @navigate-up="context.navigateUp" @navigate-down="gridRef?.focus" />
      <div
        class="flex flex-row items-center gap-1 transition duration-150 group-hover/statement:opacity-100"
        :class="context.focused.value ? '' : 'opacity-0'"
      >
        <InlineActionsCell :extra-actions="extraStatementActions" />
        <CreateFieldInterface
          ref="createFieldRef"
          :title="'New field on ' + context.statement.value.name"
          @select="createNewField"
        />
      </div>
    </div>
    <StructInterface
      ref="gridRef"
      class="-mx-1 w-full table-fixed"
      :fields="context.allFields.value"
      :model-value="context.statement.value.value"
      @update:model-value="writeValue($event)"
      @update:field="context.updateField($event, $event)"
      @delete:field="context.deleteField($event)"
      @duplicate:field="duplicateField($event)"
      :readonly="context.readonly.value"
      :active="context.focused.value"
      debounced
    />
  </div>
</template>

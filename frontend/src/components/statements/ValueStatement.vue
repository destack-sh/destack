<script lang="ts" setup>
import StructInterface from "@/components/interfaces/StructInterface.vue";
import TypedDeclarationCell from "@/components/statements/TypedDeclarationCell.vue";
import InlineActionsCell from "@/components/statements/InlineActionsCell.vue";
import type { StatementAction } from "@/state/bench";
import { useStatementContext } from "@/state/statement";
import { CubeTransparentIcon, SquaresPlusIcon } from "@heroicons/vue/24/outline";
import { computed, type Ref, ref, nextTick } from "vue";
import CreateFieldInterface from "@/components/interfaces/CreateFieldInterface.vue";

const context = useStatementContext();

const declarationRef: Ref<InstanceType<typeof TypedDeclarationCell> | null> = ref(null);
const gridRef: Ref<InstanceType<typeof StructInterface> | null> = ref(null);
const createFieldRef: Ref<InstanceType<typeof CreateFieldInterface> | null> = ref(null);

function createUnionField() {
  context.createUnionField();
  nextTick(() => declarationRef.value?.focusLastBase());
}

function createNewField(template) {}

const extraStatementActions = computed(() => {
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
      <div class="flex flex-row">
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
      :model-value="context.statement.value"
      :readonly="context.readonly.value"
      :active="context.focused.value"
      debounced
    />
  </div>
</template>

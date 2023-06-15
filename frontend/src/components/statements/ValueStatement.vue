<script lang="ts" setup>
import StructInterface from "@/components/interfaces/StructInterface.vue";
import TypedDeclarationCell from "@/components/statements/TypedDeclarationCell.vue";
import type { StatementAction } from "@/state/bench";
import { useStatementContext } from "@/state/statement";
import { CubeTransparentIcon, SquaresPlusIcon } from "@heroicons/vue/24/outline";
import { computed, type Ref, ref } from "vue";

const context = useStatementContext();

const declarationRef: Ref<InstanceType<typeof TypedDeclarationCell> | null> = ref(null);
const gridRef: Ref<InstanceType<typeof StructInterface> | null> = ref(null);

const extraStatementActions = computed(() => {
  const actions: StatementAction[] = [];
  actions.push({
    label: "Extend type",
    icon: CubeTransparentIcon,
    action: () => insertField(true),
    hideInline: true,
  });
  actions.push({
    label: "Add field",
    icon: SquaresPlusIcon,
    action: () => insertField(),
  });
  return actions;
});

defineExpose({
  focus: (position: "first" | "last" = "first") => {
    // incomplete
  },
  blur: () => {
    // incomplete
  },
});
</script>
<template>
  <div>
    <!-- Single value (vertical) -->
    <TypedDeclarationCell ref="declarationRef" @navigate-up="context.navigateUp" @navigate-down="gridRef?.focus" />
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

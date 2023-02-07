<script lang="ts" setup>
import BlankCell from "@/components/cells/BlankCell.vue";
import ModifierCell from "@/components/cells/ModifierCell.vue";
import ReferenceComboCell from "@/components/cells/ReferenceComboCell.vue";
import SymbolTypeCell from "@/components/cells/SymbolTypeCell.vue";
import { useStatementContext } from "@/components/statement";
import { ref, type Ref } from "vue";

const context = useStatementContext();

const gapRef: Ref<InstanceType<typeof BlankCell> | null> = ref(null);
const nameRef: Ref<InstanceType<typeof BlankCell> | null> = ref(null);

function deleteModifierOrSelf() {
  if (context.statement.value.modifier != null) {
    context.morphSetModifier(null);
  } else {
    context.deleteSelf();
  }
}

function deleteSymbolTypeOrModifier() {
  if (context.statement.value.symbolType != null) {
    context.morphSetSymbolType(null);
  } else {
    context.morphSetModifier(null);
  }
}

defineExpose({
  focus: () => nameRef.value?.focus(),
  defocus: () => nameRef.value?.defocus(),
});
</script>
<template>
  <span class="flex flex-row gap-1 outline-none">
    <ModifierCell v-if="context.statement.value.modifier" />
    <BlankCell
      class="-mx-0.5"
      ref="gapRef"
      @navigate-up="context.navigateUp"
      @navigate-down="context.navigateDown"
      @delete-left="deleteModifierOrSelf"
      @navigate-right="nameRef?.focus()"
    />
    <SymbolTypeCell v-if="context.statement.value.symbolType" />
    <ReferenceComboCell
      can-define-in-place
      ref="nameRef"
      @navigate-up="context.navigateUp"
      @navigate-down="context.navigateDown"
      @delete-left="deleteSymbolTypeOrModifier"
      @navigate-left="gapRef?.focus()"
    />
  </span>
</template>

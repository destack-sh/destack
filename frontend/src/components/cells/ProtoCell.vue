<script lang="ts" setup>
import SelectTypeCell from "@/components/cells/SelectTypeCell.vue";
import ModifierCell from "@/components/cells/ModifierCell.vue";
import ReferenceComboCell from "@/components/cells/ReferenceComboCell.vue";
import SymbolTypeCell from "@/components/cells/SymbolTypeCell.vue";
import { useStatementContext } from "@/components/statement";
import { ref, type Ref } from "vue";

const context = useStatementContext();

const gapRef: Ref<InstanceType<typeof SelectTypeCell> | null> = ref(null);
const nameRef: Ref<InstanceType<typeof SelectTypeCell> | null> = ref(null);

function deleteModifierOrSelf() {
  if (context.statement.value.modifier != null) {
    context.setModifier(null);
  } else {
    context.deleteSelf();
  }
}

function deleteSymbolTypeOrModifier() {
  if (context.statement.value.symbolType != null) {
    context.setSymbolType(null);
  } else {
    context.setModifier(null);
  }
}

function morphToDefinition(name: string) {
  context.morphToDefinition(context.statement.value.symbolType ?? null, name);
}

defineExpose({
  focus: () => {
    if (context.statement.value.symbolType == null) {
      // prever gap if we don't have a symbol type declared yet
      gapRef.value?.focus();
    } else {
      nameRef.value?.focus();
    }
  },
  defocus: () => {
    nameRef.value?.defocus();
    gapRef.value?.defocus();
  },
});
</script>
<template>
  <span class="flex flex-row gap-1 outline-none">
    <ModifierCell v-if="context.statement.value.modifier" />
    <SelectTypeCell
      class="-mx-0.5"
      ref="gapRef"
      @navigate-up="context.navigateUp"
      @navigate-down="context.navigateDown"
      @delete-left="deleteModifierOrSelf"
      @navigate-right="nameRef?.focus()"
    />
    <SymbolTypeCell v-if="context.statement.value.symbolType" />
    <ReferenceComboCell
      ref="nameRef"
      @navigate-up="context.navigateUp"
      @navigate-down="context.navigateDown"
      @delete-left="deleteSymbolTypeOrModifier"
      @navigate-left="gapRef?.focus()"
      :can-define-in-place="context.statement.value.symbolType != null"
      @define-in-place="morphToDefinition"
    />
  </span>
</template>

<script lang="ts" setup>
import SelectTypeCell from "@/components/cells/SelectTypeCell.vue";
import ModifierCell from "@/components/cells/ModifierCell.vue";
import ReferenceComboCell from "@/components/cells/ReferenceComboCell.vue";
import SymbolTypeCell from "@/components/cells/SymbolTypeCell.vue";
import { useStatementContext } from "@/components/statement";
import { ref, type Ref } from "vue";
import EditableSpan from "@/components/EditableSpan.vue";

const context = useStatementContext();

const startRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
const gapRef: Ref<InstanceType<typeof SelectTypeCell> | null> = ref(null);
const nameRef: Ref<InstanceType<typeof ReferenceComboCell> | null> = ref(null);

function deleteModifierOrAbove() {
  if (context.statement.value.modifier != null) {
    context.setModifier(null);
  } else {
    context.tryDeleteAbove();
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
    startRef.value?.defocus();
    nameRef.value?.defocus();
    gapRef.value?.defocus();
  },
});
</script>
<template>
  <!-- TODO @Cleanup: compress/simplify navigation across cells (proto, definition, ..) -->
  <span class="flex flex-row gap-1 outline-none">
    <EditableSpan
      :model-value="''"
      ref="startRef"
      class="-mx-0.5"
      v-if="context.statement.value.modifier != null"
      @navigate-up="context.navigateUp"
      @navigate-down="context.navigateDown"
      @navigate-right="gapRef?.focus()"
      @delete-left="context.tryDeleteAbove"
      @delete-right="context.setModifier(null)"
      @enter="context.insertAbove"
      @escape="context.escape"
      :readonly="context.readonly.value"
    />
    <ModifierCell v-if="context.statement.value.modifier" />
    <SelectTypeCell
      class="-mx-0.5"
      ref="gapRef"
      @navigate-up="context.navigateUp"
      @navigate-down="context.navigateDown"
      @delete-left="deleteModifierOrAbove"
      @navigate-left="startRef?.focus()"
      @navigate-right="nameRef?.focus()"
      @enter="context.insertAbove"
      @escape="context.escape"
      @configured="
        nameRef?.focus();
        nameRef?.open();
      "
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
      @escape="context.escape"
    />
  </span>
</template>

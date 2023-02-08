<script lang="ts" setup>
import SelectTypeCell from "@/components/cells/SelectTypeCell.vue";
import ModifierCell from "@/components/cells/ModifierCell.vue";
import SymbolTypeCell from "@/components/cells/SymbolTypeCell.vue";
import EditableSpan from "@/components/EditableSpan.vue";
import { useStatementContext } from "@/components/statement";
import { ref, type Ref } from "vue";

const context = useStatementContext();

const name: Ref<string> = ref(context.statement.value.name ?? "");
context.syncName(name);
const gapRef: Ref<InstanceType<typeof SelectTypeCell> | null> = ref(null);
const nameRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);

function deleteModifierOrSelf() {
  if (context.statement.value.modifier != null) {
    context.setModifier(null);
  } else {
    context.deleteSelf();
  }
}

defineExpose({
  focus: () => nameRef.value?.focus(),
  defocus: () => {
    nameRef.value?.defocus();
    gapRef.value?.defocus();
  },
});
</script>
<template>
  <div class="flex flex-row flex-wrap gap-1">
    <ModifierCell v-if="context.statement.value.modifier" />
    <SelectTypeCell
      class="-mx-0.5"
      ref="gapRef"
      @navigate-up="context.navigateUp"
      @navigate-down="context.navigateDown"
      @navigate-right="nameRef?.focus()"
      @delete-left="deleteModifierOrSelf"
      @escape="context.escape"
    />
    <SymbolTypeCell />
    <EditableSpan
      ref="nameRef"
      v-model="name"
      :readonly="context.readonly.value"
      @navigate-up="context.navigateUp"
      @navigate-down="context.navigateDown"
      @navigate-left="gapRef?.focus"
      @escape="context.escape"
    />
  </div>
</template>

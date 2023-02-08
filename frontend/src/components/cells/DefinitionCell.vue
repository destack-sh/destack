<script lang="ts" setup>
import SelectTypeCell from "@/components/cells/SelectTypeCell.vue";
import ModifierCell from "@/components/cells/ModifierCell.vue";
import SymbolTypeCell from "@/components/cells/SymbolTypeCell.vue";
import EditableSpan from "@/components/EditableSpan.vue";
import { useStatementContext } from "@/components/statement";
import { ref, type Ref } from "vue";
import EnumContentCell from "@/components/cells/EnumContentCell.vue";
import { TypeTag } from "@/gql/graphql";

const context = useStatementContext();

const name: Ref<string> = ref(context.statement.value.name ?? "");
context.syncName(name);
const startRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
const gapRef: Ref<InstanceType<typeof SelectTypeCell> | null> = ref(null);
const nameRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
const contentRef: Ref<InstanceType<typeof EnumContentCell> | null> = ref(null);

function deleteModifierOrAbove() {
  if (context.statement.value.modifier != null) {
    context.setModifier(null);
  } else {
    context.tryDeleteAbove();
  }
}

function navigateDown() {
  if (contentRef.value != null) {
    contentRef.value.focus();
  }
}

defineExpose({
  focus: () => nameRef.value?.focus(),
  defocus: () => {
    startRef.value?.defocus();
    nameRef.value?.defocus();
    gapRef.value?.defocus();
    contentRef.value?.defocus();
  },
});
</script>
<template>
  <div class="flex flex-row flex-wrap gap-1">
    <EditableSpan
      :model-value="''"
      ref="startRef"
      class="-mx-0.5"
      v-if="context.statement.value.modifier != null"
      @navigate-up="context.navigateUp"
      @navigate-down="navigateDown"
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
      @navigate-down="navigateDown"
      @navigate-left="startRef?.focus()"
      @navigate-right="nameRef?.focus()"
      @delete-left="deleteModifierOrAbove"
      @enter="context.insertAbove"
      @escape="context.escape"
    />
    <SymbolTypeCell />
    <EditableSpan
      ref="nameRef"
      v-model="name"
      :readonly="context.readonly.value"
      @navigate-up="context.navigateUp"
      @navigate-down="navigateDown"
      @navigate-left="gapRef?.focus"
      @escape="context.escape"
    />
  </div>
  <EnumContentCell ref="contentRef" v-if="context.typeNodeRoot.value?.tag == TypeTag.Enum" />
</template>

<script lang="ts" setup>
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import EditableSpan from "@/components/EditableSpan.vue";
import { useNavigationGrid } from "@/components/cells/grid";
import { useStatementContext } from "@/components/statement";
import { type Ref, ref } from "vue";

const context = useStatementContext();
const declarationRef: Ref<InstanceType<typeof DeclarationCell> | null> = ref(null);
const description: Ref<string> = ref(context.statement.value.description ?? "");
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
context.syncDescription(description);

function focusFirstIfExists() {
  console.log("focus first");
}

function focus() {
  declarationRef.value?.focus();
}

defineExpose({
  focus,
  blur: () => {
    declarationRef.value?.blur();
    descriptionRef.value?.blur();
  },
});
</script>
<template>
  <!-- Declaration -->
  <DeclarationCell
    ref="declarationRef"
    @navigate-down="descriptionRef?.focus"
    @navigate-right="descriptionRef?.focus"
  />
  <!-- Description -->
  <EditableSpan
    ref="descriptionRef"
    v-model="description"
    :readonly="context.readonly.value"
    @navigate-left="declarationRef?.focus()"
    @navigate-up="declarationRef?.focus()"
    @navigate-down="focusFirstIfExists"
    @enter="context.insertBelow"
  />
  <button
    tabindex="-1"
    v-if="description.length == 0"
    @click="descriptionRef?.focus()"
    class="w-fit rounded-sm px-0.5 text-gray-400 hover:bg-orange-50 hover:text-gray-700"
  >
    +description
  </button>
  <!-- TODO @Incomplete: dataset type editing -->
  <!-- TODO @Incomplete: dataset record editing -->
</template>

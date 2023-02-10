<script lang="ts" setup>
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import EditableSpan from "@/components/EditableSpan.vue";
import { useStatementContext } from "@/components/statement";
import { ref, type Ref } from "vue";

const context = useStatementContext();

const description: Ref<string> = ref(context.statement.value.description ?? "");
context.syncDescription(description);

const declarationRef: Ref<InstanceType<typeof DeclarationCell> | null> = ref(null);
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);

defineExpose({
  focus: () => declarationRef.value?.focus(),
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
    @navigate-down="descriptionRef?.focus()"
    @navigate-right="descriptionRef?.focus"
  />
  <!-- Inline type -->
  <!-- TODO @Incomplete: task type -->
  <!-- Description -->
  <EditableSpan
    ref="descriptionRef"
    v-model="description"
    :readonly="context.readonly.value"
    @navigate-left="declarationRef?.focus()"
    @navigate-up="declarationRef?.focus()"
    @navigate-down="context.navigateDown"
  />
  <button
    tabindex="-1"
    v-if="description.length == 0"
    @click="descriptionRef?.focus()"
    class="w-fit rounded-sm px-0.5 text-gray-400 hover:bg-orange-50 hover:text-gray-700"
  >
    +description
  </button>
</template>

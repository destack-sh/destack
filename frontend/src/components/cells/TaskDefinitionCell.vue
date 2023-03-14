<script lang="ts" setup>
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import InlineFunctionTypeCell from "@/components/cells/InlineFunctionTypeCell.vue";
import EditableSpan from "@/components/EditableSpan.vue";
import { useStatementContext } from "@/components/statement";
import { ref, type Ref } from "vue";

// all tasks are typed, but we currently re-use TaskDefinitionCell for expectations
// which are implicitly typed only for now
defineProps<{
  isTyped: boolean;
}>();

const context = useStatementContext();

const description: Ref<string> = ref(context.statement.value.description ?? "");
context.syncDescription(description);

const declarationRef: Ref<InstanceType<typeof DeclarationCell> | null> = ref(null);
const typeRef: Ref<InstanceType<typeof InlineFunctionTypeCell> | null> = ref(null);
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);

defineExpose({
  focus: () => declarationRef.value?.focus(),
  blur: () => {
    declarationRef.value?.blur();
    typeRef.value?.blur();
    descriptionRef.value?.blur();
  },
});
</script>
<template>
  <!-- Declaration -->
  <DeclarationCell
    ref="declarationRef"
    class="inline-flex"
    @navigate-down="descriptionRef?.focus"
    @navigate-right="typeRef?.focus"
  />
  <!-- Inline type -->
  <InlineFunctionTypeCell
    v-if="isTyped"
    ref="typeRef"
    class="ml-3 inline-flex"
    @navigate-up="context.navigateUp"
    @navigate-down="descriptionRef?.focus"
    @navigate-right="descriptionRef?.focus"
    @navigate-left="declarationRef?.focus"
  />
  <!-- Description -->
  <div>
    <EditableSpan
      ref="descriptionRef"
      v-model="description"
      :readonly="context.readonly.value"
      @navigate-left="typeRef?.focus"
      @navigate-up="declarationRef?.focus"
      @navigate-down="context.navigateDown"
      @delete-left="declarationRef?.focus"
      @enter="context.insertBelow"
    />
    <button
      tabindex="-1"
      v-if="description.trim().length == 0"
      @click="descriptionRef?.focus()"
      class="w-fit select-none rounded-sm px-0.5 text-gray-300 hover:bg-orange-100 hover:text-gray-700 group-focus-within/statement:text-gray-400"
    >
      +description
    </button>
  </div>
</template>

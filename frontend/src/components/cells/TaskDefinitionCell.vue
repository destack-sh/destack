<script lang="ts" setup>
import InlineActions from "@/components/basic/InlineActions.vue";
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import FunctionTypeCell from "@/components/cells/FunctionTypeCell.vue";
import EditableSpan from "@/components/EditableSpan.vue";
import { useStatementContext, type StatementAction } from "@/components/statement";
import { computed, ref, type Ref } from "vue";

// all tasks are typed, but we currently re-use TaskDefinitionCell for expectations
// which are implicitly typed only for now
defineProps<{
  isTyped: boolean;
}>();

const context = useStatementContext();

const description: Ref<string> = ref(context.statement.value.description ?? "");
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
context.syncDescription(
  description,
  computed(() => descriptionRef.value?.focused)
);

const declarationRef: Ref<InstanceType<typeof DeclarationCell> | null> = ref(null);
const typeRef: Ref<InstanceType<typeof FunctionTypeCell> | null> = ref(null);

const extraActions = computed(() => {
  const inlineActions: StatementAction[] = [];
  return inlineActions;
});

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
  <div class="flex flex-row justify-between">
    <DeclarationCell
      ref="declarationRef"
      class="inline-flex"
      @navigate-down="descriptionRef?.focus"
      @navigate-right="typeRef?.focus"
    />
    <InlineActions
      class="transition duration-150 group-hover/statement:opacity-100"
      :class="context.focused.value ? '' : 'opacity-0'"
      :extraActions="extraActions"
    />
  </div>
  <!-- Description -->
  <div>
    <EditableSpan
      ref="descriptionRef"
      v-model="description"
      :readonly="context.readonly.value"
      @navigate-left="typeRef?.focus"
      @navigate-up="declarationRef?.focus"
      @navigate-down="typeRef?.focus"
      @delete-left="declarationRef?.focus"
      @enter="context.insertBelow"
    />
    <button
      tabindex="-1"
      v-if="description.trim().length == 0 && !context.readonly.value"
      @click="descriptionRef?.focus()"
      class="w-fit select-none rounded-sm px-0.5 text-gray-300 hover:bg-orange-100 hover:text-gray-700 group-focus-within/statement:text-gray-400"
    >
      +description
    </button>
    <!-- Inline type -->
    <FunctionTypeCell
      v-if="isTyped && (context.typeNodes.value.length > 0 || !context.readonly.value)"
      ref="typeRef"
      class="py-1"
      @navigate-up="context.navigateUp"
      @navigate-down="context.navigateDown"
      @navigate-left="descriptionRef?.focus"
    />
  </div>
</template>

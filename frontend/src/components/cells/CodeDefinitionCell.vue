<script lang="ts" setup>
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import FunctionTypeCell from "@/components/cells/FunctionTypeCell.vue";
import MonacoEditor from "@/components/MonacoEditor.vue";
import { useStatementContext } from "@/components/statement";
import { computed, ref, type Ref } from "vue";

const context = useStatementContext();

const code: Ref<string> = ref(context.statement.value.code ?? "");
const monacoRef: Ref<InstanceType<typeof MonacoEditor> | null> = ref(null);
context.syncCode(
  code,
  computed(() => monacoRef.value?.focused)
);

const declarationRef: Ref<InstanceType<typeof DeclarationCell> | null> = ref(null);
const typeRef: Ref<InstanceType<typeof FunctionTypeCell> | null> = ref(null);
const addingTypes = ref(false);

defineExpose({
  focus: () => declarationRef.value?.focus(),
  blur: () => {
    declarationRef.value?.blur();
    typeRef.value?.blur();
    monacoRef.value?.blur();
  },
});
</script>
<template>
  <!-- Declaration -->
  <DeclarationCell
    ref="declarationRef"
    class="inline-flex"
    @navigate-down="monacoRef?.focus"
    @navigate-right="typeRef?.focus"
  />
  <!-- Inline type -->
  <button
    v-if="!context.readonly.value && context.typeNodes.value.length == 0"
    ref="typeRef"
    class="z-10 ml-2 w-fit rounded-sm px-0.5 text-sm hover:bg-orange-100 hover:text-gray-700"
    :class="context.focused.value ? 'text-gray-400' : 'text-gray-300'"
    @click="addingTypes = !addingTypes"
  >
    {{ addingTypes ? "-arguments" : "+arguments" }}
  </button>
  <FunctionTypeCell
    v-if="context.typeNodes.value.length > 0 || addingTypes"
    ref="typeRef"
    class="py-1"
    @navigate-up="context.navigateUp"
    @navigate-down="monacoRef?.focus"
    @navigate-right="monacoRef?.focus"
    @navigate-left="declarationRef?.focus"
  />
  <!-- Code -->
  <!-- TODO @UX: figure out nicer styling for code -->
  <MonacoEditor
    ref="monacoRef"
    hide-line-numbers
    :lineNumberOffset="0"
    :line-number-shift-px="context.xOffset.value - 20"
    v-model="code"
    @navigate-up="declarationRef?.focus"
    @navigate-down="context.navigateDown"
    @navigate-left="typeRef?.focus"
    @escape="context.escape"
    @enter="context.insertBelow"
    language="python"
    :focused="context.focused.value"
    :readonly="context.readonly.value"
    class="-mx-1 my-1 rounded-sm bg-gray-50 p-1"
  />
  <button
    v-if="code.trim().length == 0"
    class="absolute bottom-3 z-10 w-fit rounded-sm px-0.5 text-gray-400 hover:bg-orange-100 hover:text-gray-700"
    @click="monacoRef?.focus()"
  >
    +code
  </button>
  <!-- Last output/error (if any) -->
</template>

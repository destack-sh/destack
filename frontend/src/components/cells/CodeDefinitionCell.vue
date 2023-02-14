<script lang="ts" setup>
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import InlineFunctionTypeCell from "@/components/cells/InlineFunctionTypeCell.vue";
import MonacoEditor from "@/components/MonacoEditor.vue";
import { useStatementContext } from "@/components/statement";
import { ref, type Ref } from "vue";

const context = useStatementContext();

const code: Ref<string> = ref(context.statement.value.code ?? "");
context.syncCode(code);

const declarationRef: Ref<InstanceType<typeof DeclarationCell> | null> = ref(null);
const typeRef: Ref<InstanceType<typeof InlineFunctionTypeCell> | null> = ref(null);
const monacoRef: Ref<InstanceType<typeof MonacoEditor> | null> = ref(null);

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
  <InlineFunctionTypeCell
    ref="typeRef"
    class="ml-3 inline-flex"
    @navigate-up="context.navigateUp"
    @navigate-down="monacoRef?.focus"
    @navigate-right="monacoRef?.focus"
    @navigate-left="declarationRef?.focus"
  />
  <!-- Code -->
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
  />
  <button
    v-if="code.trim().length == 0"
    class="absolute bottom-1 z-10 w-fit rounded-sm px-0.5 text-gray-400 hover:bg-orange-50 hover:text-gray-700"
    @click="monacoRef?.focus()"
  >
    +code
  </button>
</template>

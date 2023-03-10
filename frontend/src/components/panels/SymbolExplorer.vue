<script lang="ts" setup>
import { StatementType } from "@/gql/graphql";
import { SYMBOL_TYPE_KEYWORD, useEditorState } from "@/state/editor";
import { useCurrentModuleRuntime, useSymbolNavigation } from "@/state/runtime";
import { computed } from "vue";

const runtime = useCurrentModuleRuntime();
const editor = useEditorState();
const { focusSymbol } = useSymbolNavigation();

const allSymbols = computed(() => {
  const symbols = [];

  // TODO @Cleanup: order and group symbols
  for (const file of runtime.moduleIndex.value?.module.files ?? []) {
    for (const symbol of file.symbols) {
      if (
        symbol.name == null ||
        symbol.type != StatementType.Definition ||
        (!editor.showGenerated && symbol.generated)
      ) {
        continue;
      }
      symbols.push(symbol);
    }
  }

  return symbols;
});

defineExpose({
  count: computed(() => allSymbols.value.length),
});
</script>
<template>
  <ul role="list" class="flex flex-col py-1 text-sm">
    <li
      v-for="symbol in allSymbols"
      :key="symbol.id"
      class="flex flex-row gap-1 py-0.5 px-3 text-gray-700 hover:cursor-pointer hover:bg-orange-50 hover:text-gray-900"
      :class="{ 'border-l-2 border-orange-200 pl-2.5': symbol.generated }"
      @click="focusSymbol(symbol)"
    >
      <span class="text-gray-500">{{ SYMBOL_TYPE_KEYWORD[symbol.symbolType] }}</span>
      <span class="">{{ symbol.name }}</span>
    </li>
  </ul>
</template>

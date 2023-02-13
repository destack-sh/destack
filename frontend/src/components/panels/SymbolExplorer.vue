<script lang="ts" setup>
import { StatementType, SymbolType } from "@/gql/graphql";
import { SYMBOL_TYPE_KEYWORD } from "@/state/editor";
import { useCurrentModuleRuntime } from "@/state/runtime";
import { computed } from "vue";

const runtime = useCurrentModuleRuntime();

const allSymbols = computed(() => {
  const symbols = [];

  // TODO @Cleanup: order symbols
  for (const file of runtime.moduleIndex.value?.module.files ?? []) {
    for (const symbol of file.symbols) {
      if (symbol.name == null || symbol.type != StatementType.Definition) {
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
  <ul role="list" class="flex flex-col gap-1 py-1 text-sm">
    <li v-for="symbol in allSymbols" :key="symbol.id" class="flex flex-row gap-1 px-3 text-gray-700">
      <span class="text-gray-500">{{ SYMBOL_TYPE_KEYWORD[symbol.symbolType] }}</span>
      <span class="">{{ symbol.name }}</span>
    </li>
  </ul>
</template>

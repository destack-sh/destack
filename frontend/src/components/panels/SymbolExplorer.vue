<script lang="ts" setup>
import { useNavigationGrid } from "@/components/cells/grid";
import { StatementType } from "@/gql/graphql";
import { SYMBOL_TYPE_KEYWORD, useEditorState } from "@/state/editor";
import { useCurrentModuleRuntime, useSymbolNavigation } from "@/state/runtime";
import { computed } from "vue";

const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
}>();

const runtime = useCurrentModuleRuntime();
const editor = useEditorState();
const nav = useSymbolNavigation();

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
const symbolsGrid = useNavigationGrid<"name", HTMLElement>(
  computed(() => ["name"]),
  allSymbols,
  {
    gridNavigateUp: () => emit("navigateUp"),
    gridNavigateDown: () => emit("navigateDown"),
  }
);

function focusSymbol(symbol: SymbolHeader) {
  const focusedViewId = editor.focusedViewId;
  nav.focusSymbol(symbol);
  editor.focusView(focusedViewId as ViewId); // keep focused view
}

function focusSymbolAndGoThere(symbol: SymbolHeader) {
  nav.focusSymbol(symbol);
}

function focus() {
  symbolsGrid.focus(0, "name");
}

function blur() {
  symbolsGrid.blur();
}

defineExpose({
  count: computed(() => allSymbols.value.length),
  focus,
  blur,
});
</script>
<template>
  <ul role="list" class="flex flex-col py-1 text-sm">
    <li
      v-for="symbol in allSymbols"
      :key="symbol.id"
      :ref="(ref) => symbolsGrid.registerColumnRef(symbol.id, 'name', ref)"
      tabindex="-1"
      class="flex flex-row gap-1 border border-transparent py-0.5 px-3 text-gray-700 outline-none hover:cursor-pointer hover:bg-orange-100 hover:text-gray-900 focus:border-orange-600"
      :class="{
        'border-l-2 border-orange-200 pl-2.5': symbol.generated,
        'bg-orange-100 text-orange-600': symbol.id == editor?.focusedElementId,
        'text-gray-700 hover:text-orange-600': symbol.id != editor?.focusedElementId,
      }"
      @click="focusSymbol(symbol)"
      @keydown.enter.exact.prevent="focusSymbolAndGoThere(symbol)"
      @keydown.up.exact.prevent="symbolsGrid.navigateUp(symbol.id, 'name')"
      @keydown.down.exact.prevent="symbolsGrid.navigateDown(symbol.id, 'name')"
    >
      <span class="">{{ SYMBOL_TYPE_KEYWORD[symbol.symbolType] }}</span>
      <span class="">{{ symbol.name }}</span>
    </li>
  </ul>
</template>

<script lang="ts" setup>
import { useNavigationGrid } from "@/components/cells/grid";
import { StatementType, type InterpSymbol } from "@/gql/graphql";
import { SYMBOL_TYPE_KEYWORD, useEditorState, type ViewId } from "@/state/editor";
import { useCurrentInterpModule, useSymbolNavigation } from "@/state/runtime";
import { computed, nextTick } from "vue";

const props = defineProps<{ showAllSymbols?: boolean }>();
const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
}>();

const runtime = useCurrentInterpModule();
const editor = useEditorState();
const nav = useSymbolNavigation();

const filteredSymbols = computed(() => {
  const symbols = [];

  if (!props.showAllSymbols && editor.focusedFileId == null) {
    return undefined;
  }

  // TODO @Cleanup: order and group symbols
  for (const file of runtime.moduleIndex.value?.module.files ?? []) {
    if (!props.showAllSymbols && file.id != editor.focusedFileId) {
      continue;
    }

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
  computed(() => filteredSymbols.value ?? []),
  {
    gridNavigateUp: () => emit("navigateUp"),
    gridNavigateDown: () => emit("navigateDown"),
  }
);

function focusSymbol(symbol: InterpSymbol) {
  const focusedViewId = editor.focusedViewId;
  nav.focusSymbol(symbol);
  editor.focusView(focusedViewId as ViewId); // keep focused view
  nextTick(() => nav.focusSymbol(symbol));
}

function focusSymbolAndGoThere(symbol: InterpSymbol) {
  nav.focusSymbol(symbol);
}

function focus(target: "first" | "last" = "first") {
  symbolsGrid.focus(target == "first" ? 0 : -1, "name");
}

function blur() {
  symbolsGrid.blur();
}

defineExpose({
  count: computed(() => filteredSymbols.value?.length),
  focus,
  blur,
});
</script>
<template>
  <ul v-if="filteredSymbols != null" role="list" class="flex flex-col py-1 text-sm">
    <li
      v-for="symbol in filteredSymbols"
      :key="symbol.id"
      :ref="(ref) => symbolsGrid.registerColumnRef(symbol.id, 'name', ref)"
      tabindex="-1"
      class="flex flex-row gap-1 border border-transparent px-3 py-0.5 text-gray-700 outline-none hover:cursor-pointer hover:bg-orange-100 hover:text-gray-900 focus:border-orange-600"
      :class="{
        'border-l-2 border-gray-300 pl-2.5': symbol.generated,
        'bg-orange-100 text-orange-600': symbol.id == editor?.focusedElementId,
        'text-gray-700 hover:text-orange-600': symbol.id != editor?.focusedElementId,
      }"
      @click.prevent="focusSymbol(symbol)"
      @mousedown.prevent="focusSymbol(symbol)"
      @keydown.enter.exact.prevent="focusSymbolAndGoThere(symbol)"
      @keydown.up.exact.prevent="symbolsGrid.navigateUp(symbol.id, 'name')"
      @keydown.down.exact.prevent="symbolsGrid.navigateDown(symbol.id, 'name')"
    >
      <span class="">{{ SYMBOL_TYPE_KEYWORD[symbol.symbolType] }}</span>
      <span class="" :class="editor.mainSymbolId == symbol.id ? 'font-bold' : ''">{{ symbol.name }}</span>
    </li>
  </ul>
  <div v-else class="my-2 px-3">
    <span class="text-sm text-gray-700">No file in focus.</span>
  </div>
</template>

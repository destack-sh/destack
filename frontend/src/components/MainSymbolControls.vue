<script lang="ts" setup>
import { StatementType, SymbolType, type InterpSymbol } from "@/gql/graphql";
import { provideGlobalAction } from "@/state/actions";
import { SYMBOL_TYPE_KEYWORD, useEditorState } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { fileOf, symbolsLike, useCurrentModuleRuntime } from "@/state/runtime";
import { Combobox, ComboboxButton, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { CheckBadgeIcon, ChevronUpDownIcon, PlayIcon, WrenchIcon } from "@heroicons/vue/24/outline";
import { computed, ref } from "vue";

// statement selection
const operations = useOperations();
const editor = useEditorState();
const runtime = useCurrentModuleRuntime();
const mainSymbol = computed(() => runtime.moduleIndex.value?.symbolsById[editor.mainSymbolId ?? ""]);
const mainSymbolMissing = computed(() => mainSymbol.value == null && editor.mainSymbolId != null);
const canBuild = computed(
  () =>
    runtime.connected &&
    (mainSymbol.value?.symbolType == SymbolType.Task ||
      mainSymbol.value?.symbolType == SymbolType.Build ||
      mainSymbol.value?.symbolType == SymbolType.Runconfig)
);
const canRun = computed(
  () =>
    runtime.connected &&
    (mainSymbol.value?.symbolType == SymbolType.Task || mainSymbol.value?.symbolType == SymbolType.Code)
);

const availableSymbols = symbolsLike({
  types: [StatementType.Definition],
  symbolTypes: [SymbolType.Runconfig, SymbolType.Build, SymbolType.Task, SymbolType.Code],
});
const query = ref("");
const filteredSymbols = computed(() =>
  query.value === ""
    ? availableSymbols.value
    : availableSymbols.value.filter((s) => {
        return s.name?.toLowerCase().includes(query.value.toLowerCase());
      })
);

const buildMain = provideGlobalAction({
  id: "symbol.buildMain",
  label: computed(() => "Build " + mainSymbol.value?.name),
  shortcuts: ["F6"],
  enabled: canBuild,
  apply: async () => {
    console.log("build " + mainSymbol.value?.name);
    operations.runtime.build(mainSymbol.value?.id);
  },
});

const runMain = provideGlobalAction({
  id: "symbol.runMain",
  label: computed(() => "Run " + mainSymbol.value?.name),
  shortcuts: ["F7"],
  enabled: canRun,
  apply: async () => {
    console.log("run " + mainSymbol.value?.name);
    const runEditor = editor.openRun(mainSymbol.value as any);
    editor.focusEditor(runEditor);
  },
});

const testMain = provideGlobalAction({
  id: "symbol.testMain",
  label: computed(() => "Test " + mainSymbol.value?.name),
  shortcuts: ["F8"],
  enabled: computed(() => false),
  apply: async () => {
    console.log("test");
  },
});

const mainActions = [
  {
    label: "Build",
    icon: WrenchIcon,
    enabled: canBuild,
    active: computed(() => operations.state.hasInflightLike({ types: ["runtime.build"] })),
    action: () => buildMain.value.apply(),
  },
  {
    label: "Run",
    icon: PlayIcon,
    enabled: canRun,
    active: computed(() => operations.state.hasInflightLike({ types: ["runtime.run"] })),
    action: () => runMain.value.apply(),
  },
  {
    label: "Test",
    icon: CheckBadgeIcon,
    enabled: computed(() => testMain.value.enabled),
    active: ref(false),
    action: () => testMain.value.apply(),
  },
];

function symbolDeclr(symbol: InterpSymbol | undefined) {
  if (symbol == null) {
    return null;
  } else if (symbol.symbolType != null) {
    return SYMBOL_TYPE_KEYWORD[symbol.symbolType] + " " + symbol.name;
  } else {
    return symbol.name;
  }
}
</script>
<template>
  <!-- Select main statement -->
  <Combobox
    as="div"
    class="relative"
    :model-value="mainSymbol"
    @update:model-value="(stmt) => editor.setMainSymbol(stmt)"
    nullable
  >
    <ComboboxInput
      class="max-w-fit rounded-sm border-none py-1 pl-3 pr-6 text-right text-sm font-bold text-gray-700 outline-none ring-0 placeholder:text-gray-400 focus:border-orange-500 focus:ring-0"
      @change="query = $event.target.value"
      :display-value="(stmt) => symbolDeclr(stmt)"
      :placeholder="mainSymbolMissing ? '???' : 'main...'"
      :disabled="!runtime.connected.value"
    />
    <ComboboxButton class="absolute inset-y-0 right-0 flex items-center rounded-r-md px-2 focus:outline-none">
      <ChevronUpDownIcon class="h-4 w-4 text-gray-400" aria-hidden="true" />
    </ComboboxButton>

    <ComboboxOptions
      v-show="runtime.connected.value"
      class="absolute z-10 mt-1 max-h-60 w-80 overflow-auto rounded-sm bg-white py-1 text-base shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none sm:text-sm"
    >
      <div v-if="availableSymbols.length == 0" class="py-1 px-2 text-gray-500">no runnable symbols</div>
      <div v-else-if="filteredSymbols.length == 0" class="py-1 px-2 text-gray-500">no matching symbols</div>
      <ComboboxOption
        v-for="stmt in filteredSymbols"
        :key="stmt.id"
        :value="stmt"
        as="template"
        v-slot="{ active, selected }"
      >
        <li
          :class="[
            'relative cursor-default select-none py-0.5 px-2 font-mono text-sm',
            active ? 'bg-orange-600 text-white' : 'text-gray-900',
          ]"
        >
          <div class="flex items-baseline justify-between">
            <span :class="['truncate', selected && 'underline']">
              {{ SYMBOL_TYPE_KEYWORD[stmt.symbolType] }}
              {{ stmt.name }}
            </span>
            <span class="text-xs" :class="['truncate text-gray-500', active ? 'text-orange-200' : 'text-gray-500']">
              {{ fileOf(stmt)?.path }}
            </span>
          </div>
        </li>
      </ComboboxOption>
    </ComboboxOptions>
  </Combobox>
  <button
    v-for="action in mainActions"
    :key="action.label"
    class="rounded-sm p-1 text-sm"
    :class="{
      'hover:bg-orange-50': action.enabled.value,
      'animate-pulse ': action.active.value,
    }"
    :disabled="!action.enabled.value || action.active.value"
    @click="action.action"
  >
    <component
      :is="action.icon"
      class="h-5 w-5"
      :class="{
        'text-orange-500  ': action.enabled.value,
        'text-gray-500': !action.enabled.value,
      }"
    />
  </button>
</template>

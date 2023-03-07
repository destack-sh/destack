<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { JobStatus, JobType, StatementType, SymbolType, type InterpSymbol } from "@/gql/graphql";
import { provideGlobalAction } from "@/state/actions";
import { SYMBOL_TYPE_KEYWORD, useEditorState } from "@/state/editor";
import { useNotifications } from "@/state/notifications";
import { useOperations } from "@/state/operations";
import { fileOf, symbolsLike, useCurrentModuleRuntime } from "@/state/runtime";
import { Listbox, ListboxButton, ListboxOption, ListboxOptions } from "@headlessui/vue";
import { CheckCircleIcon, ChevronDownIcon, PlayIcon, WrenchIcon } from "@heroicons/vue/24/outline";
import { computed, ref } from "vue";

// statement selection
const operations = useOperations();
const editor = useEditorState();
const runtime = useCurrentModuleRuntime();
const mainSymbol = computed(() => runtime.moduleIndex.value?.symbolsById[editor.mainSymbolId ?? ""]);
const mainSymbolMissing = computed(() => mainSymbol.value == null && editor.mainSymbolId != null);

// TODO @Cleanup: stale & active state for current main symbol are global, not specific
const mainSymbolStale = computed(() => {
  if (runtime.staleSymbols.value == null) {
    return undefined;
  } else {
    return runtime.staleSymbols.value.length > 0;
  }
});
const buildRunning = computed(
  () => runtime.jobs.value?.find((job) => job.status == JobStatus.Running && job.type == JobType.Build) != null
);
const evaluateRunning = computed(
  () => runtime.jobs.value?.find((job) => job.status == JobStatus.Running && job.type == JobType.Evaluate) != null
);

const canBuild = computed(
  () =>
    !buildRunning.value &&
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
// :ProperSymbolSearch
const filteredSymbols = computed(() =>
  query.value === ""
    ? availableSymbols.value
    : availableSymbols.value.filter((s) => {
        return s.name?.toLowerCase().includes(query.value.toLowerCase());
      })
);

const notifications = useNotifications();
const buildMain = provideGlobalAction({
  id: "symbol.buildMain",
  label: computed(() => "Build " + mainSymbol.value?.name),
  shortcuts: ["f8"],
  enabled: canBuild,
  apply: async () => {
    const ret = await operations.runtime.build(mainSymbol.value?.id);
    if (ret?.data?.build.__typename != "BuildState" || !ret.data.build.success) {
      notifications.show({
        type: "build.fail",
        kind: "error",
        message: "Build failed",
        description: `Build failed for ${mainSymbol.value?.name}`,
      });
    }
  },
});

const runMain = provideGlobalAction({
  id: "symbol.runMain",
  label: computed(() => "Run " + mainSymbol.value?.name),
  shortcuts: ["f9"],
  enabled: canRun,
  apply: async () => {
    const runEditor = editor.openRun(mainSymbol.value as any);
    editor.focusEditor(runEditor);
  },
});

const testMain = provideGlobalAction({
  id: "symbol.testMain",
  label: computed(() => "Test " + mainSymbol.value?.name),
  shortcuts: ["f10"],
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
    stale: mainSymbolStale,
    active: computed(() => buildRunning.value || operations.state.hasInflightLike({ types: ["runtime.build"] })),
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
    icon: CheckCircleIcon,
    enabled: computed(() => testMain.value.enabled),
    active: ref(false),
    stale: mainSymbolStale,
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
  <!-- Wrapper -->
  <div class="flex items-center space-x-1 rounded-sm bg-orange-100 pr-2">
    <!-- Select main statement -->
    <Listbox
      as="div"
      class="relative"
      :model-value="mainSymbol"
      @update:model-value="(stmt) => editor.setMainSymbol(stmt)"
      nullable
      v-slot="{ open }"
    >
      <ListboxButton
        class="flex w-fit max-w-fit flex-row items-center gap-1 rounded-sm border-none py-1.5 pl-4 pr-2 text-right text-sm outline-none ring-0 placeholder:text-gray-400 hover:bg-orange-200 focus:border-orange-500 focus:ring-0"
        :class="{
          'font-mono tracking-tighter': editor.fontMono,
          'text-sm': editor.textSmall,
          'text-md': !editor.textSmall,
          'text-gray-900': mainSymbol != null,
          'text-gray-700': mainSymbol == null,
          'bg-orange-200': open,
        }"
        @change="query = $event.target.value"
        @contextmenu.prevent="$event.target.click()"
        :disabled="!runtime.connected.value"
      >
        {{ mainSymbolMissing ? "???" : symbolDeclr(mainSymbol) ?? "Select" }}
        <ChevronDownIcon class="h-3 w-3 text-gray-400" aria-hidden="true" />
      </ListboxButton>

      <FadeTransition>
        <ListboxOptions
          v-show="runtime.connected.value"
          class="absolute z-10 mt-1 max-h-60 w-80 overflow-auto rounded-sm bg-white py-1 text-base shadow-md ring-1 ring-orange-900 ring-opacity-40 sm:text-sm"
          :class="{ 'font-mono': editor.fontMono, 'text-sm': editor.textSmall, 'text-md': !editor.textSmall }"
        >
          <div v-if="availableSymbols.length == 0" class="px-2 py-1 text-gray-500">No runnable symbols.</div>
          <div v-else-if="filteredSymbols.length == 0" class="px-2 py-1 text-gray-500">No matching symbols.</div>
          <ListboxOption
            v-for="stmt in filteredSymbols"
            :key="stmt.id"
            :value="stmt"
            as="template"
            v-slot="{ active, selected }"
          >
            <li
              :class="[
                'relative cursor-default select-none py-0.5 px-2',
                active ? 'bg-orange-600 text-white' : 'text-gray-900',
                selected && !active ? 'text-orange-600' : '',
              ]"
            >
              <div class="flex items-baseline justify-between">
                <span :class="['truncate']">
                  {{ SYMBOL_TYPE_KEYWORD[stmt.symbolType] }}
                  {{ stmt.name }}
                </span>
                <span class="text-xs" :class="['truncate text-gray-500', active ? 'text-orange-200' : 'text-gray-500']">
                  {{ fileOf(stmt)?.path }}
                </span>
              </div>
            </li>
          </ListboxOption>
        </ListboxOptions>
      </FadeTransition>
    </Listbox>
    <button
      v-for="action in mainActions"
      :key="action.label"
      class="relative rounded-sm p-1.5 text-sm"
      :class="{
        'text-gray-700 hover:bg-orange-200': action.enabled.value,
        'text-gray-500': !action.enabled.value,
        '': action.active.value,
      }"
      :disabled="!action.enabled.value || action.active.value"
      @click="action.action"
    >
      <component :is="action.icon" class="h-5 w-5" />
      <!-- little svg rectangle for stale/active status -->
      <svg
        v-if="mainSymbol != null && (action.active.value || action.stale != null)"
        class="absolute right-1.5 bottom-1.5 h-1 w-1 transition-all duration-100"
        :class="{
          'animate-spin text-gray-400': action.active.value,
          'text-yellow-600': !action.active.value && action.stale?.value,
          'text-transparent': !action.active.value && !action.stale?.value,
        }"
        viewBox="0 0 10 10"
        fill="none"
      >
        <rect width="10" height="10" rx="1" ry="1" fill="currentColor" />
      </svg>
    </button>
  </div>
</template>

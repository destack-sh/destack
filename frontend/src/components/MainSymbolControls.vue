<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { JobStatus, JobType, StatementType, SymbolType, type InterpSymbol, BuildScope } from "@/gql/graphql";
import { provideGlobalAction } from "@/state/actions";
import { SYMBOL_TYPE_KEYWORD, useEditorState } from "@/state/editor";
import { useJobs } from "@/state/jobs";
import { useOperations } from "@/state/operations";
import { fileOf, isSymbolStale, symbolsLike, useCurrentInterpModule, useSymbolOps } from "@/state/runtime";
import { Listbox, ListboxButton, ListboxOption, ListboxOptions } from "@headlessui/vue";
import { CheckCircleIcon, ChevronDownIcon, PlayIcon, WrenchIcon } from "@heroicons/vue/24/outline";
import { computed, ref, toRef, watchEffect } from "vue";

const props = defineProps<{
  projectId: string;
  projectVersionId: string;
}>();

// statement selection
const ops = useOperations();
const editor = useEditorState();
const runtime = useCurrentInterpModule();
const { jobs: activeJobs } = useJobs(
  {
    projectId: toRef(props, "projectId"),
    projectVersionId: toRef(props, "projectVersionId"),
    statusIn: ref([JobStatus.Running]),
  },
  { live: true }
);
const mainSymbol = computed(() => runtime.moduleIndex.value?.symbolsById[editor.mainSymbolId ?? ""]);
const mainSymbolMissing = computed(() => mainSymbol.value == null && editor.mainSymbolId != null);

const mainSymbolStale = isSymbolStale(mainSymbol);
const buildRunning = computed(() => activeJobs.value?.find((job) => job.type == JobType.Build) != null);
const evaluateRunning = computed(() => activeJobs.value?.find((job) => job.type == JobType.Evaluate) != null);

const canBuild = computed(
  () =>
    !buildRunning.value &&
    runtime.connected &&
    (mainSymbol.value?.symbolType == SymbolType.Task ||
      mainSymbol.value?.symbolType == SymbolType.Build ||
      mainSymbol.value?.symbolType == SymbolType.Runconfig)
);
const canRun = computed(
  () => mainSymbol.value?.symbolType == SymbolType.Task || mainSymbol.value?.symbolType == SymbolType.Code
);

const availableSymbols = symbolsLike({
  types: [StatementType.Definition, StatementType.Redefinition],
  symbolTypes: [SymbolType.Task],
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

// auto-select first available symbol
watchEffect(() => {
  if (mainSymbol.value == null && availableSymbols.value.length > 0 && !editor.mainSymbolUnset) {
    editor.setMainSymbol(availableSymbols.value[0]);
  }
});

const symbolOps = useSymbolOps();
const buildMain = provideGlobalAction({
  id: "symbol.buildMain",
  label: computed(() => "Build " + mainSymbol.value?.name),
  shortcuts: ["f8"],
  enabled: canBuild,
  apply: async () => {
    if (mainSymbol.value == null) return;
    await symbolOps.build(mainSymbol.value, BuildScope.Reactive);
  },
});

const runMain = provideGlobalAction({
  id: "symbol.runMain",
  label: computed(() => "Run " + mainSymbol.value?.name),
  shortcuts: ["f9"],
  enabled: canRun,
  apply: async () => {
    if (mainSymbol.value == null) return;
    await symbolOps.openRun(mainSymbol.value);
  },
});

const evaluateMain = provideGlobalAction({
  id: "symbol.evaluateMain",
  label: computed(() => "Evaluate " + mainSymbol.value?.name),
  shortcuts: ["f10"],
  enabled: computed(() => true),
  apply: async () => {
    if (mainSymbol.value == null) return;
    await symbolOps.evaluate(mainSymbol.value);
  },
});

const mainActions = [
  {
    label: "Build",
    icon: WrenchIcon,
    enabled: canBuild,
    stale: mainSymbolStale,
    active: computed(() => buildRunning.value || ops.state.hasInflightLike({ types: ["runtime.build"] })),
    action: () => buildMain.value.apply(),
  },
  {
    label: "Run",
    icon: PlayIcon,
    enabled: canRun,
    active: computed(() => ops.state.hasInflightLike({ types: ["runtime.run"] })),
    action: () => runMain.value.apply(),
  },
  {
    label: "Evaluate",
    icon: CheckCircleIcon,
    enabled: computed(() => evaluateMain.value.enabled),
    active: computed(
      () => evaluateRunning.value || ops.state.hasInflightLike({ types: ["runtime.build", "runtime.evaluate"] })
    ),
    stale: mainSymbolStale,
    action: () => evaluateMain.value.apply(),
  },
];
</script>
<template>
  <!-- Wrapper -->
  <div
    class="flex flex-row items-center space-x-1 rounded-sm border border-orange-900 border-opacity-[12%] bg-orange-100 pr-2"
    v-if="availableSymbols.length > 0"
  >
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
        class="flex w-fit max-w-fit flex-row items-center gap-1 whitespace-nowrap rounded-sm border-none py-1.5 pl-4 pr-2 text-right text-sm outline-none ring-0 placeholder:text-gray-400 hover:bg-orange-200 focus:border-orange-500 focus:ring-0"
        :class="{
          'font-mono tracking-tighter': editor.fontMono,
          'text-sm': editor.textSmall,
          'text-md': !editor.textSmall,
          'text-gray-800': mainSymbol != null,
          'text-gray-700': mainSymbol == null,
          'bg-orange-200': open,
          'font-semibold': !mainSymbolMissing,
        }"
        @change="query = $event.target.value"
        @contextmenu.prevent="$event.target.click()"
        :disabled="!runtime.connected.value"
      >
        {{ mainSymbolMissing ? "???" : mainSymbol?.name ?? runtime.name.value ?? "???" }}
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
          <!-- Deselect (null option) -->
          <ListboxOption :key="null" :value="null" as="template" v-slot="{ active, selected }">
            <li
              :class="[
                'relative cursor-default select-none px-2 py-0.5',
                active ? 'bg-orange-600 text-white' : 'text-gray-900',
                selected && !active ? 'text-orange-600' : '',
              ]"
            >
              <div class="flex items-baseline justify-between">
                <span class="truncate text-gray-500">[{{ runtime.name.value }}]</span>
              </div>
            </li>
          </ListboxOption>
          <!-- Actual  options -->
          <ListboxOption
            v-for="stmt in filteredSymbols"
            :key="stmt.id"
            :value="stmt"
            as="template"
            v-slot="{ active, selected }"
          >
            <li
              :class="[
                'relative cursor-default select-none px-2 py-0.5',
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
    <template v-if="!editor.readonly">
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
          class="absolute bottom-1.5 right-1.5 h-1 w-1 transition-all duration-100"
          :class="{
            'animate-spin text-gray-400': action.active.value,
            'text-transparent': !action.active.value,
          }"
          viewBox="0 0 10 10"
          fill="none"
        >
          <rect width="10" height="10" rx="1" ry="1" fill="currentColor" />
        </svg>
      </button>
    </template>
  </div>
</template>

<script lang="ts" setup>
import { StatementType, SymbolType } from "@/gql/graphql";
import { SYMBOL_TYPE_KEYWORD, useEditorState } from "@/state/editor";
import { fileOf, statementsLike, useCurrentModuleRuntime } from "@/state/runtime";
import { Combobox, ComboboxButton, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { BeakerIcon, CheckIcon, ChevronUpDownIcon, PlayIcon, WrenchIcon } from "@heroicons/vue/24/outline";
import { computed, ref, type Ref } from "vue";

// statement selection
const editor = useEditorState();
const runtime = useCurrentModuleRuntime();
const mainStatement = computed(() => runtime.moduleIndex.value?.statementsByGlobalId[editor.mainStatementId ?? ""]);

const availableStatements = statementsLike({
  types: [StatementType.Definition],
  symbolTypes: [SymbolType.Runconfig, SymbolType.Compilation],
});
const query = ref("");
const filteredStatements = computed(() =>
  query.value === ""
    ? availableStatements.value
    : availableStatements.value.filter((s) => {
        return s.name?.toLowerCase().includes(query.value.toLowerCase());
      })
);

const mainActions = [
  {
    label: "Compile",
    icon: WrenchIcon,
    enabled: true,
    active: true,
    action: async () => {
      console.log("compile");
    },
  },
  {
    label: "Run",
    icon: PlayIcon,
    enabled: true,
    active: false,
    action: async () => {
      console.log("run");
    },
  },
  {
    label: "Test",
    icon: BeakerIcon,
    enabled: false,
    active: false,
    action: async () => {
      console.log("test");
    },
  },
];
</script>
<template>
  <!-- Select main statement -->
  <Combobox
    as="div"
    class="relative"
    :model-value="mainStatement"
    @update:model-value="(stmt) => editor.setMainStatement({ id: stmt.globalId })"
    nullable
  >
    <ComboboxInput
      class="max-w-fit rounded-sm border border-gray-300 py-1 pl-3 pr-10 font-mono outline-none ring-0 focus:border-orange-500 focus:ring-0 sm:text-sm"
      @change="query = $event.target.value"
      :display-value="(stmt) => stmt?.name"
      placeholder="select main..."
    />
    <ComboboxButton class="absolute inset-y-0 right-0 flex items-center rounded-r-md px-2 focus:outline-none">
      <ChevronUpDownIcon class="h-4 w-4 text-gray-400" aria-hidden="true" />
    </ComboboxButton>

    <ComboboxOptions
      v-if="filteredStatements.length > 0"
      class="absolute z-10 mt-1 max-h-60 w-full overflow-auto rounded-sm bg-white py-1 text-base shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none sm:text-sm"
    >
      <ComboboxOption
        v-for="stmt in filteredStatements"
        :key="stmt.id"
        :value="stmt"
        as="template"
        v-slot="{ active, selected }"
      >
        <li
          :class="[
            'relative cursor-default select-none py-0.5 pl-3 pr-9 font-mono text-sm',
            active ? 'bg-orange-600 text-white' : 'text-gray-900',
          ]"
        >
          <div class="flex items-baseline justify-between">
            <span :class="['truncate', selected && 'font-semibold']">
              <span v-if="stmt.symbolType != null">
                {{ SYMBOL_TYPE_KEYWORD[stmt.symbolType] }}
              </span>
              {{ stmt.name }}
            </span>
            <span class="text-xs" :class="['truncate text-gray-500', active ? 'text-orange-200' : 'text-gray-500']">
              {{ fileOf(stmt)?.path }}
            </span>
          </div>

          <span
            v-if="selected"
            :class="['absolute inset-y-0 right-0 flex items-center pr-2', active ? 'text-white' : 'text-orange-600']"
          >
            <CheckIcon class="h-4 w-4" aria-hidden="true" />
          </span>
        </li>
      </ComboboxOption>
    </ComboboxOptions>
  </Combobox>
  <button
    v-for="action in mainActions"
    :key="action.label"
    class="rounded-sm p-1 text-sm"
    :class="{
      'hover:bg-orange-50': action.enabled,
      'animate-pulse ': action.active,
    }"
    :disabled="!action.enabled || action.active"
    @click="action.action"
  >
    <component
      :is="action.icon"
      class="h-5 w-5"
      :class="{
        'text-orange-500  ': action.enabled,
        'text-gray-500': !action.enabled,
      }"
    />
  </button>
</template>

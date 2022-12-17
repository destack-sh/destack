<script lang="ts" setup>
import MonacoEditor from "@/components/MonacoEditor.vue";
import { useFragment, type FragmentType } from "@/gql";
import type { DatasetContentFragment } from "@/gql/graphql";
import { DatasetContentType, useDatasetInterfaceState } from "@/utils/dataset";
import { SymbolContentType } from "@/utils/fragments";
import { useSchemadSymbolSchema } from "@/utils/intellisense";
import { computed } from "vue";

const props = defineProps<{
  symbol: FragmentType<typeof SymbolContentType>;
  content: FragmentType<typeof DatasetContentType>;
  compiled: boolean;
  commented: boolean;
  focused: boolean;
  lineNumberBase: number;
  xOffset: number;
}>();

const symbol = computed(() => useFragment(SymbolContentType, props.symbol));
const content = computed(() => useFragment(DatasetContentType, props.content));

function datasetToJsonObj(content: DatasetContentFragment) {
  return content.records.map((r) => r.data);
}

const contentAsJsonObj = computed(() => datasetToJsonObj(content.value));
const contentAsJsonText = computed(() => JSON.stringify(contentAsJsonObj.value, null, 2));
const contentAsJsonlText = computed(() => contentAsJsonObj.value.map((r) => JSON.stringify(r)).join("\n"));

// sense derived state
const { schemaElement } = useSchemadSymbolSchema(symbol);
const schemaElements = computed(() => schemaElement.value?.elements ?? []);
const schemaAvailable = computed(() => schemaElement.value != null);

// local interface state
const state = useDatasetInterfaceState(symbol);
</script>
<template>
  <div class="flex h-full w-full flex-col">
    <!-- Data view -->
    <MonacoEditor
      v-if="state.view == 'jsonl' || state.view == 'json'"
      :line-number-offset="lineNumberBase + 1 /* for statement itself */"
      :line-number-shift-px="xOffset + 20"
      :style="{ marginLeft: -xOffset - 44 + 'px' }"
      :model-value="state.view == 'jsonl' ? contentAsJsonlText : contentAsJsonText"
      language="json"
      :focused="focused"
      :readonly="compiled"
    />
    <table
      v-else-if="state.view == 'table'"
      class="h-full w-full rounded-sm"
      :class="{ ' divide-y divide-gray-300': state.showTableHeader }"
    >
      <thead class="bg-gray-50" v-show="state.showTableHeader && schemaAvailable">
        <tr>
          <th
            v-for="element in schemaElements"
            :key="element.name"
            class="py-1.5 pr-2 text-left text-sm font-normal text-black"
          >
            {{ element.name }}
          </th>
        </tr>
      </thead>
      <tbody class="divide-y divide-gray-200">
        <tr class="relative" v-for="record in content.records" :key="record.index">
          <template v-if="schemaAvailable">
            <td
              v-for="element in schemaElements"
              :key="element.name"
              class="whitespace-pre-wrap py-1 pr-2 align-top text-sm text-black"
            >
              {{ record.data[element.name] || "" }}
            </td>
          </template>
          <td
            v-else
            class="animate-pulse whitespace-pre-wrap rounded-sm bg-gray-50 py-1 pr-2 text-center align-top text-sm text-gray-50"
          >
            <!-- invisible placeholder if schema is invalid / loading  -->
            ...
          </td>
          <!-- Imitate Monaco line numbers -->
          <span
            class="absolute top-1 w-6 select-none text-right font-mono text-sm"
            :style="{ left: -xOffset - 42 + 'px' }"
            :class="{ 'text-orange-200': !focused, 'text-orange-400': focused }"
            >{{ lineNumberBase + 1 + record.index + 1 }}</span
          >
        </tr>
      </tbody>
    </table>
  </div>
</template>

<script lang="ts" setup>
import MonacoEditor from "@/components/MonacoEditor.vue";
import { useFragment, type FragmentType } from "@/gql";
import type { DatasetContentFragment } from "@/gql/graphql";
import { DatasetContentType, useDatasetInterfaceState } from "@/state/dataset";
import { FileHeaderType, StatementContentType } from "@/state/fragments";
import { computed } from "vue";

const props = defineProps<{
  file: FragmentType<typeof FileHeaderType>;
  statement: FragmentType<typeof StatementContentType>;
  content: FragmentType<typeof DatasetContentType>;
  focused: boolean;
  readonly: boolean;
  editing: boolean;
  lineNumberBase: number;
  xOffset: number;
}>();
const emit = defineEmits<{
  (e: "navigateUp", position?: number): void;
  (e: "navigateDown", position?: number): void;
  (e: "escape"): void;
}>();

const file = computed(() => useFragment(FileHeaderType, props.file));
const statement = computed(() => useFragment(StatementContentType, props.statement));
const content = computed(() => useFragment(DatasetContentType, props.content));

function datasetToJsonObj(content: DatasetContentFragment) {
  return content.records.map((r) => r.data);
}

const contentAsJsonObj = computed(() => datasetToJsonObj(content.value));
const contentAsJsonText = computed(() => JSON.stringify(contentAsJsonObj.value, null, 2));
const contentAsJsonlText = computed(() => contentAsJsonObj.value.map((r) => JSON.stringify(r)).join("\n"));

// TODO @Incomplete: data type info
const typeAvailable = computed(() => false);
const typeNodes = computed(() => []);

// local interface state
const state = useDatasetInterfaceState(statement);
</script>
<template>
  <div class="flex h-full w-full flex-col">
    <!-- Data view -->
    <MonacoEditor
      v-if="state.view == 'jsonl' || state.view == 'json'"
      :line-number-offset="lineNumberBase + 1 /* for statement itself */"
      :line-number-shift-px="xOffset + 20"
      :hide-line-numbers="state.view == 'json' /* json has inaccurate line numbers */"
      :style="{ marginLeft: -xOffset - (20 + (state.view == 'jsonl' ? 23 : 0)) + 'px' }"
      :model-value="state.view == 'jsonl' ? contentAsJsonlText : contentAsJsonText"
      language="json"
      :focused="focused"
      :readonly="readonly"
      @navigateUp="emit('navigateUp')"
      @navigateDown="emit('navigateDown')"
      @escape="emit('escape')"
    />
    <table
      v-else-if="state.view == 'table'"
      class="h-full w-full rounded-sm"
      :class="{ ' divide-y divide-gray-300': state.showTableHeader }"
    >
      <thead class="bg-gray-50" v-show="state.showTableHeader && typeAvailable">
        <tr>
          <th v-for="node in typeNodes" :key="node.name" class="py-1.5 pr-2 text-left text-sm font-normal text-black">
            {{ node.name }}
          </th>
        </tr>
      </thead>
      <tbody class="divide-y divide-gray-200">
        <tr class="relative" v-for="(i, record) in content.records" :key="i">
          <template v-if="typeAvailable">
            <td
              v-for="node in typeNodes"
              :key="node.name"
              class="whitespace-pre-wrap py-1 pr-2 align-top text-sm text-black"
            >
              {{ record.data[node.name] || "" }}
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
            >{{ lineNumberBase + 1 + record + 1 }}</span
          >
        </tr>
      </tbody>
    </table>
  </div>
</template>

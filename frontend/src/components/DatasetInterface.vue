<script lang="ts" setup>
import MonacoEditor from "@/components/MonacoEditor.vue";
import { useFragment, type FragmentType } from "@/gql";
import { useDatasetInterfaceState } from "@/state/dataset";
import { FileHeaderType, StatementContentType } from "@/state/fragments";
import { useOperations } from "@/state/operations";
import { useRuntimeTypeOf } from "@/state/runtime";
import { useDebounce, useDebounceFn } from "@vueuse/core";
import { computed, ref } from "vue";

const props = defineProps<{
  file: FragmentType<typeof FileHeaderType>;
  statement: FragmentType<typeof StatementContentType>;
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

const statement = computed(() => useFragment(StatementContentType, props.statement));
const monacoEditor = ref<InstanceType<typeof MonacoEditor> | null>(null);

const recordsData = computed(() => statement.value.records.map((r) => r.data));
const recordsAsJsonlText = computed(() => recordsData.value.map((r) => JSON.stringify(r)).join("\n"));
const recordsAsCsvText = computed(() => {
  const records = statement.value.records;
  const keys = Object.keys(records[0].data);
  const lines = [];
  for (const record of records) {
    const values = keys.map((k) => record.data[k]);
    lines.push(values.join(","));
  }
  return lines.join("\n");
});

const typeNode = useRuntimeTypeOf(statement);

const operations = useOperations();
async function saveRecords(records: Array<JSON>) {
  await operations.content.updateStatementRecords(
    statement.value.id,
    statement.value.records.map((r) => r.data) ?? [],
    records
  );
}

function saveRecordsFromString(records: string) {
  if (state.value.view == "csv") {
    throw new Error("csv save not implemented");
  } else if (state.value.view == "jsonl") {
    // parse each line the jsonl string to form a json array
    const lines = records.split("\n");
    saveRecords(lines.map((line) => JSON.parse(line)));
  } else {
    throw new Error("unexpected view in save: " + state.value.view);
  }
}

const saveRecordsFromStringDebounced = useDebounceFn(saveRecordsFromString, 200, { maxWait: 500 });

// local interface state
const state = useDatasetInterfaceState(statement);

defineExpose({
  focus: () => monacoEditor.value?.focus(),
  blur: () => monacoEditor.value?.blur(),
});
</script>
<template>
  <!-- Data view -->
  <MonacoEditor
    ref="monacoEditor"
    v-if="state.view == 'jsonl' || state.view == 'csv'"
    :line-number-offset="lineNumberBase + 1 /* for statement itself */"
    :line-number-shift-px="xOffset + 20"
    :style="{ marginLeft: -xOffset - 43 + 'px' }"
    :model-value="{ jsonl: recordsAsJsonlText, csv: recordsAsCsvText }[state.view]"
    @update:model-value="saveRecordsFromStringDebounced"
    @navigateUp="emit('navigateUp')"
    @navigateDown="emit('navigateDown')"
    @escape="emit('escape')"
    :language="{ jsonl: 'json', csv: 'csv' }[state.view]"
    :focused="focused"
    :readonly="props.readonly || state.view == 'csv'"
  />
  <table
    v-else-if="state.view == 'table'"
    class="h-full w-full rounded-sm"
    :class="{ ' divide-y divide-gray-300': state.showTableHeader }"
  >
    <thead class="bg-gray-50" v-if="state.showTableHeader && typeNode != null">
      <tr>
        <th
          v-for="node in typeNode.children"
          :key="node.name"
          class="py-1.5 pr-2 text-left text-sm font-normal text-black"
        >
          {{ node.name }}
        </th>
      </tr>
    </thead>
    <tbody class="divide-y divide-gray-200">
      <tr class="relative" v-for="(record, i) in statement.records" :key="i">
        <template v-if="typeNode != null">
          <td
            v-for="node in typeNode.children"
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
          >{{ lineNumberBase + 1 + i + 1 }}</span
        >
      </tr>
    </tbody>
  </table>
</template>

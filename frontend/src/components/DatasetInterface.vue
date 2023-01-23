<script lang="ts" setup>
import MonacoEditor from "@/components/MonacoEditor.vue";
import { useFragment, type FragmentType } from "@/gql";
import { useDatasetInterfaceState } from "@/state/dataset";
import { FileHeaderType, StatementContentType } from "@/state/fragments";
import { useCurrentModuleRuntime, useRuntimeTypeOf } from "@/state/runtime";
import { computed, ref, toRef } from "vue";

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

const dataAsJson = computed(() => statement.value.records.map((r) => r.data));
const dataAsJsonText = computed(() => JSON.stringify(dataAsJson.value, null, 2));
const dataAsJsonlText = computed(() => dataAsJson.value.map((r) => JSON.stringify(r)).join("\n"));
const dataAsCsvText = computed(() => {
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

// local interface state
const state = useDatasetInterfaceState(statement);

defineExpose({
  focus: () => monacoEditor.value?.focus(),
  defocus: () => monacoEditor.value?.defocus(),
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
    :model-value="{ json: dataAsJsonText, jsonl: dataAsJsonlText, csv: dataAsCsvText }[state.view]"
    @navigateUp="emit('navigateUp')"
    @navigateDown="emit('navigateDown')"
    @escape="emit('escape')"
    :language="{ jsonl: 'json', csv: 'csv' }[state.view]"
    :focused="focused"
    :readonly="props.readonly"
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

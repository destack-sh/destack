<script lang="ts" setup>
import ListboxSelect from "@/components/basic/ListboxSelect.vue";
import MonacoEditor from "@/components/MonacoEditor.vue";
import { graphql, useFragment, type FragmentType } from "@/gql";
import type { DatasetContentFragment } from "@/gql/graphql";
import { SchemaElementContentDeepType } from "@/utils/fragments";
import { computed, ref, type Ref } from "vue";

const DatasetContentType = graphql(/* GraphQL */ `
  fragment DatasetContent on Dataset {
    id
    schema {
      ...SchemaElementContentDeep
    }
    records {
      data
      index
    }
  }
`);

const props = defineProps<{ content: FragmentType<typeof DatasetContentType>; generated: boolean; focused: boolean }>();
const content = computed(() => useFragment(DatasetContentType, props.content));
const schema = computed(() => useFragment(SchemaElementContentDeepType, content.value.schema));
const schemaElements = computed(() => schema.value?.elements || []);

type ViewMode = "table" | "json" | "jsonl";
const viewOptions: { name: string; value: ViewMode }[] = [
  { name: "Table", value: "table" },
  { name: "JSONL", value: "jsonl" },
  { name: "JSON", value: "json" },
];
const viewMode: Ref<any> = ref(viewOptions[0]);

function datasetToJsonObj(content: DatasetContentFragment) {
  return content.records.map((r) => r.data);
}

const contentAsJsonObj = computed(() => datasetToJsonObj(content.value));
const contentAsJsonText = computed(() => JSON.stringify(contentAsJsonObj.value, null, 2));
const contentAsJsonlText = computed(() => contentAsJsonObj.value.map((r) => JSON.stringify(r)).join("\n"));
</script>
<template>
  <div class="flex h-full w-full flex-col">
    <!-- Schema & controls -->
    <div class="mx-2 my-1 flex flex-row justify-between">
      <div class="flex flex-row gap-2">
        <span v-for="element in schemaElements" :key="element.name" class="text-sm text-gray-700">
          {{ element.name }} <span class="text-gray-500">({{ element.type.toLowerCase() }})</span>
        </span>
      </div>
      <ListboxSelect class="max-w-fit" v-model="viewMode" :options="viewOptions"> </ListboxSelect>
    </div>

    <!-- Data view -->
    <MonacoEditor
      v-if="viewMode.value == 'jsonl'"
      :model-value="contentAsJsonlText"
      language="json"
      :focused="focused"
      :readonly="generated"
    />
    <MonacoEditor
      v-else-if="viewMode.value == 'json'"
      :model-value="contentAsJsonText"
      language="json"
      :focused="focused"
      :readonly="generated"
    />
    <table v-else-if="viewMode.value == 'table'" class="h-full w-full divide-y divide-gray-300 rounded-sm">
      <thead class="bg-gray-50">
        <tr>
          <th
            v-for="element in schemaElements"
            :key="element.name"
            class="px-2 py-2.5 text-left text-sm font-semibold text-gray-900"
          >
            {{ element.name }}
          </th>
        </tr>
      </thead>
      <tbody class="divide-y divide-gray-200">
        <tr v-for="record in content.records" :key="record.index">
          <td
            v-for="element in schemaElements"
            :key="element.name"
            class="whitespace-nowrap px-2 py-2 text-sm text-gray-900"
          >
            {{ record.data[element.name] }}
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

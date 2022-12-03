<script lang="ts" setup>
import ListboxSelect from "@/components/basic/ListboxSelect.vue";
import MonacoEditor from "@/components/MonacoEditor.vue";
import { graphql, useFragment, type FragmentType } from "@/gql";
import type { DatasetContentFragment } from "@/gql/graphql";
import { computed, ref, type Ref } from "vue";

const DatasetContent = graphql(/* GraphQL */ `
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

const props = defineProps<{ content: FragmentType<typeof DatasetContent>; generated: boolean; focused: boolean }>();
const content = computed(() => useFragment(DatasetContent, props.content));

type ViewMode = "table" | "json" | "jsonl";
const viewOptions: { name: string; value: ViewMode }[] = [
  { name: "Table", value: "table" },
  { name: "JSON", value: "json" },
  { name: "JSONL", value: "jsonl" },
];
const viewMode: Ref<any> = ref(viewOptions[2]);

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
    <div class="mb-1 flex flex-row justify-between">
      <div class="text-gray-900">
        {{ content.schema }}
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
  </div>
</template>

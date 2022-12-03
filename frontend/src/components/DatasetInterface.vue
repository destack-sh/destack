<script lang="ts" setup>
import MonacoEditor from "@/components/MonacoEditor.vue";
import { graphql, useFragment, type FragmentType } from "@/gql";
import type { DatasetContentFragment } from "@/gql/graphql";
import { computed } from "vue";

const DatasetContent = graphql(/* GraphQL */ `
  fragment DatasetContent on Dataset {
    id
    records {
      data
      index
    }
  }
`);

const props = defineProps<{ content: FragmentType<typeof DatasetContent>; generated: boolean; focused: boolean }>();
const content = computed(() => useFragment(DatasetContent, props.content));

function datasetToJsonObj(content: DatasetContentFragment) {
  return content.records.map((r) => r.data);
}

const contentAsJsonObj = computed(() => datasetToJsonObj(content.value));
</script>
<template>
  <div class="h-full w-full">
    <MonacoEditor
      :model-value="JSON.stringify(contentAsJsonObj, null, 2)"
      language="json"
      :focused="focused"
      :readonly="generated"
    />
  </div>
</template>

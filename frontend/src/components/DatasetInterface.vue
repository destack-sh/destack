<script lang="ts" setup>
import MonacoEditor from "@/components/MonacoEditor.vue";
import { graphql, useFragment, type FragmentType } from "@/gql";
import { computed } from "vue";

const DatasetContentFragment = graphql(/* GraphQL */ `
  fragment DatasetContent on Dataset {
    id
    records {
      data
      index
    }
  }
`);
type DatasetContent = FragmentType<typeof DatasetContentFragment>;

const props = defineProps<{ content: DatasetContent }>();
const content = useFragment(DatasetContentFragment, props.content);

function datasetToJsonObj(content: DatasetContent) {
  return content.records.map((r) => r.data);
}

const asJsonObj = computed(() => datasetToJsonObj(props.content));
</script>
<template>
  <div class="h-full w-full">
    <MonacoEditor :model-value="JSON.stringify(asJsonObj, null, 2)" language="json" />
  </div>
</template>

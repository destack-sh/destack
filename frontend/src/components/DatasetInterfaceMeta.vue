<script lang="ts" setup>
import ListboxSelect from "@/components/basic/ListboxSelect.vue";
import SchemaElement from "@/components/SchemaElement.vue";
import { useFragment, type FragmentType } from "@/gql";
import { DatasetContentType, useDatasetInterfaceState, viewModes, type ViewMode } from "@/utils/dataset";
import { FileHeaderType, StatementContentType } from "@/utils/fragments";
import { useSchemadSymbolSchema } from "@/utils/intellisense";
import { computed } from "vue";

const props = defineProps<{
  file: FragmentType<typeof FileHeaderType>;
  statement: FragmentType<typeof StatementContentType>;
  content: FragmentType<typeof DatasetContentType>;
}>();

const file = computed(() => useFragment(FileHeaderType, props.file));
const statement = computed(() => useFragment(StatementContentType, props.statement));
const content = computed(() => useFragment(DatasetContentType, props.content));
const state = useDatasetInterfaceState(statement);

function viewToObject(view: string) {
  return {
    name: view,
    value: view,
  };
}
const viewAsObject = computed(() => viewToObject(state.value.view));
const availableViews = computed(() => viewModes.map(viewToObject));

const { schemaContent } = useSchemadSymbolSchema(file, statement);
</script>
<template>
  <div class="inline-flex flex-row items-baseline gap-2 text-xs">
    <SchemaElement
      :element="schemaContent?.element"
      class="text-xs opacity-50 group-hover:opacity-100"
      v-if="schemaContent"
    />
    <span v-else class="italic text-yellow-500">no schema</span>
    <span class="text-gray-500">{{ content.length }}</span>
    <ListboxSelect
      class="-mr-1 max-w-fit"
      :model-value="viewAsObject"
      @update:model-value="state = { ...state, view: $event.value as ViewMode }"
      :options="availableViews"
    />
  </div>
</template>

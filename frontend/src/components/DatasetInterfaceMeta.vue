<script lang="ts" setup>
import ListboxSelect from "@/components/basic/ListboxSelect.vue";
import { useFragment, type FragmentType } from "@/gql";
import { DatasetContentType, useDatasetInterfaceState, viewModes, type ViewMode } from "@/state/dataset";
import { FileHeaderType, StatementContentType } from "@/state/fragments";
import { computed } from "vue";

const props = defineProps<{
  file: FragmentType<typeof FileHeaderType>;
  statement: FragmentType<typeof StatementContentType>;
  content: FragmentType<typeof DatasetContentType>;
}>();

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
</script>
<template>
  <div class="inline-flex flex-row items-baseline gap-2 text-xs">
    <span class="text-gray-500">{{ content.records.length }}</span>
    <ListboxSelect
      class="-mr-1 max-w-fit"
      :model-value="viewAsObject"
      @update:model-value="state = { ...state, view: $event.value as ViewMode }"
      :options="availableViews"
    />
  </div>
</template>

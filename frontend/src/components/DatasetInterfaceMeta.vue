<script lang="ts" setup>
import ListboxSelect from "@/components/basic/ListboxSelect.vue";
import { useFragment, type FragmentType } from "@/gql";
import { DatasetContentType, useDatasetInterfaceState, viewModes, type ViewMode } from "@/utils/dataset";
import { SymbolContentType } from "@/utils/fragments";
import { computed } from "vue";

const props = defineProps<{
  symbol: FragmentType<typeof SymbolContentType>;
  content: FragmentType<typeof DatasetContentType>;
}>();

const symbol = computed(() => useFragment(SymbolContentType, props.symbol));
const content = computed(() => useFragment(DatasetContentType, props.content));
const state = useDatasetInterfaceState(symbol);

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
  <div class="inline-flex flex-row items-baseline gap-2">
    <span class="text-xs">
      <span class="font-bold">{{ content.length }}</span> records
    </span>
    <ListboxSelect
      class="-mr-1 max-w-fit text-xs"
      :model-value="viewAsObject"
      @update:model-value="state = { ...state, view: $event.value as ViewMode }"
      :options="availableViews"
    />
  </div>
</template>

<script lang="ts" setup>
import ListboxSelect from "@/components/basic/ListboxSelect.vue";
import SchemaElement from "@/components/SchemaElement.vue";
import { useFragment, type FragmentType } from "@/gql";
import { DatasetContentType, useDatasetInterfaceState, viewModes, type ViewMode } from "@/utils/dataset";
import { SymbolContentType } from "@/utils/fragments";
import { useSchemadSymbolSchema } from "@/utils/intellisense";
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

const { schema } = useSchemadSymbolSchema(symbol);
</script>
<template>
  <div class="inline-flex flex-row items-baseline gap-2 text-xs">
    <span class="">
      <span class="text-gray-500">{{ content.length }}</span> records
    </span>
    <SchemaElement :element="schema.element" class="text-xs opacity-50 group-hover:opacity-100" v-if="schema" />
    <ListboxSelect
      class="-mr-1 max-w-fit"
      :model-value="viewAsObject"
      @update:model-value="state = { ...state, view: $event.value as ViewMode }"
      :options="availableViews"
    />
  </div>
</template>

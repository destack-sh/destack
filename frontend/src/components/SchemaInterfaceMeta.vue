<script lang="ts" setup>
import ListboxSelect from "@/components/basic/ListboxSelect.vue";
import { useFragment, type FragmentType } from "@/gql";
import { StatementContentType } from "@/utils/fragments";
import { useSchemaInterfaceState, viewModes, type SchemaContentType, type ViewMode } from "@/utils/schema";
import { computed } from "vue";

const props = defineProps<{
  statement: FragmentType<typeof StatementContentType>;
  content: FragmentType<typeof SchemaContentType>;
}>();

const statement = computed(() => useFragment(StatementContentType, props.statement));
const state = useSchemaInterfaceState(statement);

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
    <ListboxSelect
      class="-mr-1 max-w-fit text-xs"
      :model-value="viewAsObject"
      @update:model-value="state = { ...state, view: $event.value as ViewMode }"
      :options="availableViews"
    />
  </div>
</template>

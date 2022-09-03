<template>
  <ListboxSelect
    label="Dataset handler"
    :model-value="modelValue"
    with-check
    @update:model-value="(selected) => $emit('update:modelValue', selected)"
    :options="metaStore.datasetHandlers"
  >
  </ListboxSelect>
</template>
<script lang="ts" setup>
import { useMetaStore } from "@/stores";
import type { DatasetHandlerSpec } from "@/types/spec";
import ListboxSelect from "@/components/basic/ListboxSelect.vue";

const metaStore = useMetaStore();

defineProps<{ modelValue: DatasetHandlerSpec | null }>();
defineEmits<{ (e: "update:modelValue", value: DatasetHandlerSpec): void }>();
</script>

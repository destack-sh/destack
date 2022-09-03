<template>
  <ListboxSelect
    label="Model handler"
    :model-value="modelValue"
    with-check
    @update:model-value="(selected) => $emit('update:modelValue', selected)"
    :options="metaStore.modelHandlers"
  >
  </ListboxSelect>
</template>
<script lang="ts" setup>
import { useMetaStore } from "@/stores";
import type { ModelHandlerSpec } from "@/types/spec";
import ListboxSelect from "@/components/basic/ListboxSelect.vue";

const metaStore = useMetaStore();

defineProps<{ modelValue: ModelHandlerSpec | null }>();
defineEmits<{ (e: "update:modelValue", value: ModelHandlerSpec): void }>();
</script>

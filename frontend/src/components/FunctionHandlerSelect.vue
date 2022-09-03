<template>
  <ListboxSelect
    label="Function handler"
    :model-value="modelValue"
    with-check
    @update:model-value="(selected) => $emit('update:modelValue', selected)"
    :options="metaStore.functionHandlers"
  >
  </ListboxSelect>
</template>
<script lang="ts" setup>
import { useMetaStore } from "@/stores";
import type { FunctionHandlerSpec } from "@/types/spec";
import ListboxSelect from "@/components/basic/ListboxSelect.vue";

const metaStore = useMetaStore();

defineProps<{ modelValue: FunctionHandlerSpec | null }>();
defineEmits<{ (e: "update:modelValue", value: FunctionHandlerSpec): void }>();
</script>

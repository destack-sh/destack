<template>
  <input
    type="number"
    :value="modelValue"
    @input="$emit('update:modelValue', $event.target?.value)"
    class="block w-full rounded-md border-gray-300 shadow-sm focus:border-orange-500 focus:ring-orange-500 sm:text-sm"
    :placeholder="`${placeholder}`"
  />
</template>
<script lang="ts" setup>
import { watchEffect } from "vue";

const props = defineProps<{
  modelValue?: number;
  default?: number;
  placeholder?: number;
}>();
const emit = defineEmits(["update:modelValue"]);

watchEffect(() => {
  if (props.modelValue == null && props.default != null) {
    emit("update:modelValue", props.default);
  }
});
</script>

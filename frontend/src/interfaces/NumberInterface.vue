<template>
  <input
    type="number"
    class="block w-full rounded-md border-gray-300 shadow-sm focus:border-orange-500 focus:ring-orange-500 sm:text-sm"
    :value="modelValue"
    @input="$emit('update:modelValue', Number.parseFloat($event.target?.value))"
    :placeholder="`${props.default}`"
  />
</template>
<script lang="ts" setup>
import { ref, watchEffect } from "vue";

const props = defineProps<{
  modelValue?: number;
  default?: number;
  dtype?: string;
}>();
const emit = defineEmits(["update:modelValue"]);

const setDefault = ref(false);
watchEffect(() => {
  if (setDefault.value) {
    return;
  }
  setDefault.value = true;

  if (props.modelValue == null || Number.isNaN(props.modelValue)) {
    if (props.default != null && !Number.isNaN(props.default)) {
      emit("update:modelValue", props.default);
    } else {
      emit("update:modelValue", 1);
    }
  }
});
</script>

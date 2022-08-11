<template>
  <input
    type="text"
    :value="modelValue"
    @input="$emit('update:modelValue', $event.target?.value)"
    class="block w-full rounded-md border-gray-300 shadow-sm focus:border-orange-500 focus:ring-orange-500 sm:text-sm"
    :placeholder="placeholder"
  />
</template>
<script lang="ts" setup>
import { ref, watchEffect } from "vue";

const props = defineProps<{
  modelValue?: string;
  default?: string;
  placeholder?: string;
}>();
const emit = defineEmits(["update:modelValue"]);

const setDefault = ref(false);
watchEffect(() => {
  if (setDefault.value) {
    return;
  }
  setDefault.value = true;
  if (props.modelValue == null && props.default != null) {
    emit("update:modelValue", props.default);
  }
});
</script>

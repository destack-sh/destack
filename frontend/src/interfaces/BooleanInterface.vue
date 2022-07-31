<template>
  <div class="flex h-5 items-center">
    <input
      id="poll"
      aria-describedby="poll-description"
      name="poll"
      type="checkbox"
      class="h-4 w-4 rounded border-gray-300 text-orange-600 focus:ring-orange-500"
      :value="modelValue"
      @input="$emit('update:modelValue', $event.target?.value)"
    />
  </div>
</template>
<script lang="ts" setup>
import { watchEffect } from "vue";

const props = defineProps<{
  modelValue?: boolean;
  default?: boolean;
}>();
const emit = defineEmits(["update:modelValue"]);

watchEffect(() => {
  if (props.modelValue == null) {
    if (props.default != null) {
      emit("update:modelValue", props.default);
    } else {
      emit("update:modelValue", false);
    }
  }
});
</script>

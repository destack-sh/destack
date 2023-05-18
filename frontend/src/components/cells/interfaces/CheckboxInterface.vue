<script lang="ts" setup>
import { ref } from "vue";

const props = defineProps<{
  modelValue: boolean;
  readonly: boolean;
  preview: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: boolean): void;
}>();

defineExpose({
  click: () => {
    if (!props.readonly) {
      console.log("click", props.modelValue);
      emit("update:modelValue", !props.modelValue);
    }
  },
});
</script>
<template>
  <div>
    <!-- Wrapper element to forward appropriate properties -->
    <input
      type="checkbox"
      class="h-4 w-4 rounded border border-gray-300 text-orange-600 ring-0 hover:cursor-pointer focus:ring-0"
      :checked="modelValue"
      @change="
        $event.stopPropagation();
        emit('update:modelValue', $event.target?.checked);
      "
      :disabled="props.readonly"
    />
  </div>
</template>

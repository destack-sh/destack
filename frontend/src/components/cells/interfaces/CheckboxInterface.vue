<script lang="ts" setup>
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
      emit("update:modelValue", !props.modelValue);
    }
  },
});
</script>
<template>
  <input
    type="checkbox"
    class="h-4 w-4 rounded border-gray-300 text-orange-600 focus:ring-orange-600"
    :checked="modelValue"
    @change="emit('update:modelValue', $event.target?.checked)"
    :disabled="props.readonly"
  />
</template>

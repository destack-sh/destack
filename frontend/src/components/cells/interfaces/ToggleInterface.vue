<script lang="ts" setup>
import Switch from "@/components/basic/Switch.vue";

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
  <Switch
    type="checkbox"
    class="h-4 w-4 rounded border-gray-300 text-orange-600 focus:ring-orange-600"
    :model-value="modelValue"
    @update:model-value="emit('update:modelValue', $event)"
    :disabled="props.readonly"
  />
</template>

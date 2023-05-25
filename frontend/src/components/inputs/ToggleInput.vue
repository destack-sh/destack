<script lang="ts" setup>
import Switch from "@/components/basic/Switch.vue";
import { ref } from "vue";

const props = defineProps<{
  modelValue: boolean;
  readonly: boolean;
  preview: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: boolean): void;
}>();

const checkboxRef = ref<InstanceType<typeof Switch> | null>(null);

defineExpose({
  click: () => {
    if (!props.readonly) {
      emit("update:modelValue", !props.modelValue);
    }
  },
  focus: () => {
    /* noop */
  },
  blur: () => {
    /* noop */
  },
});
</script>
<template>
  <Switch
    type="checkbox"
    ref="checkboxRef"
    class="my-1 h-4 w-4 rounded border-gray-300 text-orange-600 focus:ring-orange-600"
    :model-value="modelValue"
    @update:model-value="emit('update:modelValue', $event)"
    :disabled="props.readonly"
    @click.stop
  />
</template>

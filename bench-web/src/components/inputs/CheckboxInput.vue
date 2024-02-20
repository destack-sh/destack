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

const checkboxRef = ref<HTMLInputElement | null>(null);

defineExpose({
  click: () => {
    if (!props.readonly) {
      emit("update:modelValue", !props.modelValue);
    }
  },
  focus: () => checkboxRef.value?.focus(),
  blur: () => checkboxRef.value?.blur(),
});
</script>
<template>
  <div>
    <!-- Wrapper element to forward appropriate properties only -->
    <input
      type="checkbox"
      ref="checkboxRef"
      class="h-4 w-4 rounded border border-gray-300 text-orange-600 ring-0 hover:cursor-pointer focus:ring-0"
      :checked="modelValue"
      @change="
        $event.stopPropagation();
        emit('update:modelValue', ($event.target as any)?.checked);
      "
      :disabled="props.readonly"
      @click.stop
    />
  </div>
</template>

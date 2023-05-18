<script lang="ts" setup>
const props = defineProps<{
  modelValue: any;
  readonly: boolean;
  preview: boolean;
  active: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: any): void;
}>();
</script>
<template>
  <div v-if="preview">secret preview</div>
  <input v-else type="password" :value="modelValue" @input="emit('update:modelValue', $event.target?.value)" />
</template>
